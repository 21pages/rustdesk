use std::{
    ffi::{c_void, OsStr},
    io, mem,
    os::windows::ffi::OsStrExt,
    ptr, slice,
};

use winapi::{
    shared::{
        dxgiformat::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R16G16B16A16_FLOAT},
        dxgitype::DXGI_SAMPLE_DESC,
        minwindef::HMODULE,
        winerror::S_OK,
    },
    um::{
        d3d11::{
            ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, ID3D11PixelShader,
            ID3D11RenderTargetView, ID3D11SamplerState, ID3D11ShaderResourceView,
            ID3D11Texture2D as NativeTexture, ID3D11VertexShader, D3D11_BIND_CONSTANT_BUFFER,
            D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BUFFER_DESC,
            D3D11_COMPARISON_NEVER, D3D11_CPU_ACCESS_READ, D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            D3D11_FLOAT32_MAX, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SAMPLER_DESC,
            D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC, D3D11_TEXTURE_ADDRESS_CLAMP,
            D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING, D3D11_VIEWPORT,
        },
        d3dcommon::{ID3DBlob, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST},
        libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryW},
    },
};

use super::{wrap_hresult, ComPtr};

const TONE_MAP_SHADER: &[u8] = br#"
Texture2D<float4> input_texture : register(t0);
SamplerState input_sampler : register(s0);

cbuffer ToneMapConfig : register(b0) {
    float inv_sdr_white_multiplier;
    float3 config_padding;
};

struct VertexOutput {
    float4 position : SV_POSITION;
    float2 tex_coord : TEXCOORD0;
};

VertexOutput VSMain(uint vertex_id : SV_VertexID) {
    VertexOutput output;
    float2 tex_coord = float2((vertex_id << 1) & 2, vertex_id & 2);
    output.position = float4(tex_coord * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
    output.tex_coord = tex_coord;
    return output;
}

float SoftShoulder(float value) {
    static const float knee = 0.75;
    if (value <= knee) {
        return value;
    }
    return knee + (1.0 - knee) *
           (1.0 - exp(-(value - knee) / (1.0 - knee)));
}

float3 LinearToSrgb(float3 value) {
    value = saturate(value);
    float3 low = 12.92 * value;
    float3 high = 1.055 * pow(value, 1.0 / 2.4) - 0.055;
    return lerp(high, low, step(value, 0.0031308));
}

float4 PSMain(VertexOutput input) : SV_TARGET {
    float3 linear_rgb = max(input_texture.Sample(input_sampler, input.tex_coord).rgb, 0.0) *
                        inv_sdr_white_multiplier;
    float peak = max(linear_rgb.r, max(linear_rgb.g, linear_rgb.b));
    if (peak > 0.000001) {
        linear_rgb *= SoftShoulder(peak) / peak;
    }
    float3 srgb = LinearToSrgb(linear_rgb);
    float noise = frac(52.9829189 * frac(dot(input.position.xy,
                        float2(0.06711056, 0.00583715)))) - 0.5;
    return float4(saturate(srgb + noise / 255.0), 1.0);
}
"#;

pub(super) struct ToneMapper {
    device: *mut ID3D11Device,
    context: ComPtr<ID3D11DeviceContext>,
    vertex_shader: ComPtr<ID3D11VertexShader>,
    pixel_shader: ComPtr<ID3D11PixelShader>,
    sampler: ComPtr<ID3D11SamplerState>,
    config_buffer: ComPtr<ID3D11Buffer>,
    input_copy: ComPtr<NativeTexture>,
    output: ComPtr<NativeTexture>,
    staging: ComPtr<NativeTexture>,
    render_target: ComPtr<ID3D11RenderTargetView>,
    sdr_white_multiplier: f32,
    width: u32,
    height: u32,
    data: Vec<u8>,
    logged: bool,
    logged_readback: bool,
}

#[repr(C)]
struct ToneMapConfig {
    inv_sdr_white_multiplier: f32,
    padding: [f32; 3],
}

impl ToneMapper {
    pub(super) fn new(device: *mut ID3D11Device, sdr_white_multiplier: f32) -> io::Result<Self> {
        if device.is_null() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut context = ptr::null_mut();
        unsafe { (*device).GetImmediateContext(&mut context) };
        if context.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "ID3D11Device::GetImmediateContext returned null",
            ));
        }
        let context = ComPtr(context);
        let vertex_bytecode = compile_shader(TONE_MAP_SHADER, b"VSMain\0", b"vs_4_0\0")?;
        let pixel_bytecode = compile_shader(TONE_MAP_SHADER, b"PSMain\0", b"ps_4_0\0")?;

        let mut vertex_shader = ptr::null_mut();
        let mut pixel_shader = ptr::null_mut();
        unsafe {
            wrap_hresult((*device).CreateVertexShader(
                vertex_bytecode.as_ptr() as *const _,
                vertex_bytecode.len(),
                ptr::null_mut(),
                &mut vertex_shader,
            ))?;
        }
        let vertex_shader = ComPtr(vertex_shader);
        unsafe {
            wrap_hresult((*device).CreatePixelShader(
                pixel_bytecode.as_ptr() as *const _,
                pixel_bytecode.len(),
                ptr::null_mut(),
                &mut pixel_shader,
            ))?;
        }
        let pixel_shader = ComPtr(pixel_shader);

        let mut sampler_desc: D3D11_SAMPLER_DESC = unsafe { mem::zeroed() };
        sampler_desc.Filter = D3D11_FILTER_MIN_MAG_MIP_LINEAR;
        sampler_desc.AddressU = D3D11_TEXTURE_ADDRESS_CLAMP;
        sampler_desc.AddressV = D3D11_TEXTURE_ADDRESS_CLAMP;
        sampler_desc.AddressW = D3D11_TEXTURE_ADDRESS_CLAMP;
        sampler_desc.ComparisonFunc = D3D11_COMPARISON_NEVER;
        sampler_desc.MaxLOD = D3D11_FLOAT32_MAX;
        let mut sampler = ptr::null_mut();
        unsafe { wrap_hresult((*device).CreateSamplerState(&sampler_desc, &mut sampler))? };
        let sampler = ComPtr(sampler);

        let sdr_white_multiplier = if sdr_white_multiplier.is_finite() {
            sdr_white_multiplier.clamp(0.1, 20.0)
        } else {
            1.0
        };
        let config = ToneMapConfig {
            inv_sdr_white_multiplier: 1.0 / sdr_white_multiplier,
            padding: [0.0; 3],
        };
        let config_desc = D3D11_BUFFER_DESC {
            ByteWidth: mem::size_of::<ToneMapConfig>() as u32,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_CONSTANT_BUFFER,
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: 0,
        };
        let config_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: &config as *const _ as *const _,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };
        let mut config_buffer = ptr::null_mut();
        unsafe {
            wrap_hresult((*device).CreateBuffer(&config_desc, &config_data, &mut config_buffer))?
        };

        Ok(Self {
            device,
            context,
            vertex_shader,
            pixel_shader,
            sampler,
            config_buffer: ComPtr(config_buffer),
            input_copy: ComPtr(ptr::null_mut()),
            output: ComPtr(ptr::null_mut()),
            staging: ComPtr(ptr::null_mut()),
            render_target: ComPtr(ptr::null_mut()),
            sdr_white_multiplier,
            width: 0,
            height: 0,
            data: Vec::new(),
            logged: false,
            logged_readback: false,
        })
    }

    pub(super) fn render(
        &mut self,
        input: *mut NativeTexture,
        width: u32,
        height: u32,
    ) -> io::Result<*mut NativeTexture> {
        if input.is_null() || width == 0 || height == 0 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.ensure_output(width, height)?;
        let input_view = self.create_input_view(input)?;

        let viewport = D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: width as f32,
            Height: height as f32,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };
        let input_view_ptr = input_view.0;
        let sampler_ptr = self.sampler.0;
        let config_buffer_ptr = self.config_buffer.0;
        let render_target_ptr = self.render_target.0;
        unsafe {
            (*self.context.0).IASetInputLayout(ptr::null_mut());
            (*self.context.0).IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            (*self.context.0).VSSetShader(self.vertex_shader.0, ptr::null_mut(), 0);
            (*self.context.0).PSSetShader(self.pixel_shader.0, ptr::null_mut(), 0);
            (*self.context.0).PSSetShaderResources(0, 1, &input_view_ptr);
            (*self.context.0).PSSetSamplers(0, 1, &sampler_ptr);
            (*self.context.0).PSSetConstantBuffers(0, 1, &config_buffer_ptr);
            (*self.context.0).RSSetViewports(1, &viewport);
            (*self.context.0).OMSetRenderTargets(1, &render_target_ptr, ptr::null_mut());
            (*self.context.0).Draw(3, 0);

            let null_view: *mut ID3D11ShaderResourceView = ptr::null_mut();
            let null_target: *mut ID3D11RenderTargetView = ptr::null_mut();
            (*self.context.0).PSSetShaderResources(0, 1, &null_view);
            (*self.context.0).OMSetRenderTargets(1, &null_target, ptr::null_mut());
        }

        if !self.logged {
            hbb_common::log::info!(
                "======== HDR SDR tone-map: input=R16G16B16A16Float, output=BGRA8, curve=sdr-white-normalized-soft-shoulder, sdr_white_multiplier={:.3}, sdr_white_nits={:.1}, size={}x{}",
                self.sdr_white_multiplier,
                80.0 * self.sdr_white_multiplier,
                width,
                height
            );
            self.logged = true;
        }
        Ok(self.output.0)
    }

    #[cfg(test)]
    fn map(&mut self, input: *mut NativeTexture, width: u32, height: u32) -> io::Result<&[u8]> {
        let output = self.render(input, width, height)?;
        self.readback(output)
    }

    pub(super) fn readback(&mut self, input: *mut NativeTexture) -> io::Result<&[u8]> {
        if input.is_null() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut input_desc: D3D11_TEXTURE2D_DESC = unsafe { mem::zeroed() };
        unsafe { (*input).GetDesc(&mut input_desc) };
        if input_desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected tone-map output format: {}", input_desc.Format),
            ));
        }
        self.ensure_staging(&input_desc)?;
        unsafe {
            (*self.context.0).CopyResource(self.staging.0 as *mut _, input as *mut _);
        }

        let mut mapped: D3D11_MAPPED_SUBRESOURCE = unsafe { mem::zeroed() };
        unsafe {
            wrap_hresult((*self.context.0).Map(
                self.staging.0 as *mut _,
                0,
                D3D11_MAP_READ,
                0,
                &mut mapped,
            ))?;
            let len = mapped.RowPitch as usize * input_desc.Height as usize;
            self.data.resize(len, 0);
            self.data
                .copy_from_slice(slice::from_raw_parts(mapped.pData as *const u8, len));
            (*self.context.0).Unmap(self.staging.0 as *mut _, 0);
        }

        if !self.logged_readback {
            hbb_common::log::info!(
                "======== HDR SDR readback: output=BGRA8, size={}x{}, stride={}",
                input_desc.Width,
                input_desc.Height,
                mapped.RowPitch
            );
            self.logged_readback = true;
        }
        Ok(&self.data)
    }

    fn ensure_staging(&mut self, input_desc: &D3D11_TEXTURE2D_DESC) -> io::Result<()> {
        let mut current_desc: D3D11_TEXTURE2D_DESC = unsafe { mem::zeroed() };
        if !self.staging.is_null() {
            unsafe { (*self.staging.0).GetDesc(&mut current_desc) };
        }
        if !self.staging.is_null()
            && current_desc.Width == input_desc.Width
            && current_desc.Height == input_desc.Height
            && current_desc.Format == input_desc.Format
        {
            return Ok(());
        }

        let mut desc = *input_desc;
        desc.MipLevels = 1;
        desc.ArraySize = 1;
        desc.SampleDesc.Count = 1;
        desc.SampleDesc.Quality = 0;
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
        desc.MiscFlags = 0;
        let mut staging = ptr::null_mut();
        unsafe {
            wrap_hresult((*self.device).CreateTexture2D(&desc, ptr::null(), &mut staging))?;
        }
        self.staging = ComPtr(staging);
        Ok(())
    }

    fn create_input_view(
        &mut self,
        input: *mut NativeTexture,
    ) -> io::Result<ComPtr<ID3D11ShaderResourceView>> {
        let mut input_desc: D3D11_TEXTURE2D_DESC = unsafe { mem::zeroed() };
        unsafe { (*input).GetDesc(&mut input_desc) };
        if input_desc.Format != DXGI_FORMAT_R16G16B16A16_FLOAT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected HDR texture format: {}", input_desc.Format),
            ));
        }

        let mut input_view = ptr::null_mut();
        if input_desc.BindFlags & D3D11_BIND_SHADER_RESOURCE != 0
            && input_desc.ArraySize == 1
            && unsafe {
                (*self.device).CreateShaderResourceView(
                    input as *mut _,
                    ptr::null(),
                    &mut input_view,
                )
            } == S_OK
        {
            return Ok(ComPtr(input_view));
        }

        let mut current_desc: D3D11_TEXTURE2D_DESC = unsafe { mem::zeroed() };
        if !self.input_copy.is_null() {
            unsafe { (*self.input_copy.0).GetDesc(&mut current_desc) };
        }
        if self.input_copy.is_null()
            || current_desc.Width != input_desc.Width
            || current_desc.Height != input_desc.Height
            || current_desc.Format != input_desc.Format
        {
            let mut copy_desc = input_desc;
            copy_desc.MipLevels = 1;
            copy_desc.ArraySize = 1;
            copy_desc.SampleDesc.Count = 1;
            copy_desc.SampleDesc.Quality = 0;
            copy_desc.Usage = D3D11_USAGE_DEFAULT;
            copy_desc.BindFlags = D3D11_BIND_SHADER_RESOURCE;
            copy_desc.CPUAccessFlags = 0;
            copy_desc.MiscFlags = 0;
            let mut input_copy = ptr::null_mut();
            unsafe {
                wrap_hresult((*self.device).CreateTexture2D(
                    &copy_desc,
                    ptr::null(),
                    &mut input_copy,
                ))?;
            }
            self.input_copy = ComPtr(input_copy);
        }
        unsafe {
            (*self.context.0).CopyResource(self.input_copy.0 as *mut _, input as *mut _);
            input_view = ptr::null_mut();
            wrap_hresult((*self.device).CreateShaderResourceView(
                self.input_copy.0 as *mut _,
                ptr::null(),
                &mut input_view,
            ))?;
        }
        Ok(ComPtr(input_view))
    }

    fn ensure_output(&mut self, width: u32, height: u32) -> io::Result<()> {
        if self.width == width && self.height == height && !self.output.is_null() {
            return Ok(());
        }

        let mut desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let mut output = ptr::null_mut();
        let mut render_target = ptr::null_mut();
        let mut staging = ptr::null_mut();
        unsafe {
            wrap_hresult((*self.device).CreateTexture2D(&desc, ptr::null(), &mut output))?;
            let output = ComPtr(output);
            wrap_hresult((*self.device).CreateRenderTargetView(
                output.0 as *mut _,
                ptr::null(),
                &mut render_target,
            ))?;
            let render_target = ComPtr(render_target);
            desc.Usage = D3D11_USAGE_STAGING;
            desc.BindFlags = 0;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
            wrap_hresult((*self.device).CreateTexture2D(&desc, ptr::null(), &mut staging))?;
            let staging = ComPtr(staging);
            self.output = output;
            self.render_target = render_target;
            self.staging = staging;
        }
        self.width = width;
        self.height = height;
        Ok(())
    }
}

type D3DCompile = unsafe extern "system" fn(
    *const c_void,
    usize,
    *const i8,
    *const c_void,
    *mut c_void,
    *const i8,
    *const i8,
    u32,
    u32,
    *mut *mut ID3DBlob,
    *mut *mut ID3DBlob,
) -> i32;

fn compile_shader(
    source: &[u8],
    entry: &'static [u8],
    target: &'static [u8],
) -> io::Result<Vec<u8>> {
    let compiler = LoadedLibrary::new("d3dcompiler_47.dll")
        .or_else(|_| LoadedLibrary::new("d3dcompiler_43.dll"))?;
    let compile: D3DCompile = unsafe { mem::transmute(compiler.function(b"D3DCompile\0")?) };
    let mut bytecode = ptr::null_mut();
    let mut errors = ptr::null_mut();
    let result = unsafe {
        compile(
            source.as_ptr() as *const c_void,
            source.len(),
            ptr::null(),
            ptr::null(),
            ptr::null_mut(),
            entry.as_ptr() as *const i8,
            target.as_ptr() as *const i8,
            (1 << 11) | (1 << 15),
            0,
            &mut bytecode,
            &mut errors,
        )
    };
    let errors = ComPtr(errors);
    if result != S_OK {
        let details = if errors.is_null() {
            String::new()
        } else {
            unsafe {
                String::from_utf8_lossy(slice::from_raw_parts(
                    (*errors.0).GetBufferPointer() as *const u8,
                    (*errors.0).GetBufferSize(),
                ))
                .into_owned()
            }
        };
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("D3DCompile failed: {result:#X} {details}"),
        ));
    }
    let bytecode = ComPtr(bytecode);
    let data = unsafe {
        slice::from_raw_parts(
            (*bytecode.0).GetBufferPointer() as *const u8,
            (*bytecode.0).GetBufferSize(),
        )
        .to_vec()
    };
    Ok(data)
}

struct LoadedLibrary {
    module: HMODULE,
    name: &'static str,
}

impl LoadedLibrary {
    fn new(name: &'static str) -> io::Result<Self> {
        let wide_name: Vec<u16> = OsStr::new(name).encode_wide().chain(Some(0)).collect();
        let module = unsafe { LoadLibraryW(wide_name.as_ptr()) };
        if module.is_null() {
            let error = io::Error::last_os_error();
            return Err(io::Error::new(
                error.kind(),
                format!("LoadLibraryW({name}) failed: {error}"),
            ));
        }
        Ok(Self { module, name })
    }

    fn function(&self, name: &'static [u8]) -> io::Result<*const c_void> {
        let function = unsafe { GetProcAddress(self.module, name.as_ptr() as _) };
        if function.is_null() {
            let error = io::Error::last_os_error();
            return Err(io::Error::new(
                error.kind(),
                format!("GetProcAddress({}) failed: {error}", self.name),
            ));
        }
        Ok(function as *const c_void)
    }
}

impl Drop for LoadedLibrary {
    fn drop(&mut self) {
        if unsafe { FreeLibrary(self.module) } == 0 {
            hbb_common::log::debug!(
                "Failed to unload {}: {}",
                self.name,
                io::Error::last_os_error()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winapi::um::{
        d3d11::{D3D11CreateDevice, D3D11_SDK_VERSION, D3D11_SUBRESOURCE_DATA},
        d3dcommon::D3D_DRIVER_TYPE_WARP,
    };

    #[test]
    fn tone_map_shader_compiles() {
        compile_shader(TONE_MAP_SHADER, b"VSMain\0", b"vs_4_0\0").unwrap();
        compile_shader(TONE_MAP_SHADER, b"PSMain\0", b"ps_4_0\0").unwrap();
    }

    #[test]
    fn tone_map_renders_bgra() {
        let mut device = ptr::null_mut();
        let mut context = ptr::null_mut();
        let result = unsafe {
            D3D11CreateDevice(
                ptr::null_mut(),
                D3D_DRIVER_TYPE_WARP,
                ptr::null_mut(),
                0,
                ptr::null(),
                0,
                D3D11_SDK_VERSION,
                &mut device,
                ptr::null_mut(),
                &mut context,
            )
        };
        assert_eq!(result, S_OK);
        let device = ComPtr(device);
        let _context = ComPtr(context);

        let pixels = [
            0x3c00u16, 0, 0, 0x3c00, 0, 0x3c00, 0, 0x3c00, 0, 0, 0x3c00, 0x3c00, 0x3c00, 0x3c00,
            0x3c00,
        ];
        let desc = D3D11_TEXTURE2D_DESC {
            Width: 2,
            Height: 2,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R16G16B16A16_FLOAT,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let initial = D3D11_SUBRESOURCE_DATA {
            pSysMem: pixels.as_ptr() as *const _,
            SysMemPitch: 16,
            SysMemSlicePitch: 0,
        };
        let mut texture = ptr::null_mut();
        let result = unsafe { (*device.0).CreateTexture2D(&desc, &initial, &mut texture) };
        assert_eq!(result, S_OK);
        let texture = ComPtr(texture);

        let mut tone_mapper = ToneMapper::new(device.0, 1.0).unwrap();
        let output = tone_mapper.map(texture.0, 2, 2).unwrap();
        assert!(output[2] > 200 && output[0] < 10 && output[1] < 10);
        assert!(output[5] > 200 && output[4] < 10 && output[6] < 10);

        let mut normalized_tone_mapper = ToneMapper::new(device.0, 2.0).unwrap();
        let normalized = normalized_tone_mapper.map(texture.0, 2, 2).unwrap();
        assert!(normalized[2] > 175 && normalized[2] < 200);
        assert!(normalized[5] > 175 && normalized[5] < 200);
    }
}
