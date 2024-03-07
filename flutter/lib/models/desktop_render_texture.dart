import 'package:flutter/material.dart';
import 'package:flutter_gpu_texture_renderer/flutter_gpu_texture_renderer.dart';
import 'package:flutter_hbb/consts.dart';
import 'package:flutter_hbb/models/model.dart';
import 'package:get/get.dart';
import 'package:texture_rgba_renderer/texture_rgba_renderer.dart';

import '../../common.dart';
import './platform_model.dart';

final useTextureRender =
    bind.mainHasPixelbufferTextureRender() || bind.mainHasGpuTextureRender();

class _PixelbufferTexture {
  int _textureKey = -1;
  int _display = 0;
  SessionID? _sessionId;
  final support = bind.mainHasPixelbufferTextureRender();
  bool _destroying = false;
  int? _id;

  final textureRenderer = TextureRgbaRenderer();

  int get display => _display;

  create(int d, SessionID sessionId, FFI ffi) async {
    if (support) {
      _display = d;
      _textureKey = bind.getNextTextureKey();
      _sessionId = sessionId;
      try {
        int id = await textureRenderer.createTexture(_textureKey);
        if (id != -1) {
          _id = id;
          ffi.textureModel.setRgbaTextureId(display: d, id: id);
          final ptr = await textureRenderer.getTexturePtr(_textureKey);
          platformFFI.registerPixelbufferTexture(sessionId, display, ptr);
          debugPrint(
              "create pixelbuffer texture: peerId: ${ffi.id} display:$_display, textureId:$id");
        }
      } catch (e) {
        debugPrint('create pixelbuffer texture failed: $e');
      }
    }
  }

  destroy(bool unregisterTexture, FFI ffi) async {
    if (!_destroying && support && _textureKey != -1 && _sessionId != null) {
      _destroying = true;
      if (unregisterTexture) {
        platformFFI.registerPixelbufferTexture(_sessionId!, display, 0);
        // sleep for a while to avoid the texture is used after it's unregistered.
        await Future.delayed(Duration(milliseconds: 100));
      }
      await textureRenderer.closeTexture(_textureKey);
      _textureKey = -1;
      _destroying = false;
      debugPrint(
          "destroy pixelbuffer texture: peerId: ${ffi.id} display:$_display, textureId:$_id");
    }
  }
}

class _GpuTexture {
  int _textureId = -1;
  SessionID? _sessionId;
  final support = bind.mainHasGpuTextureRender();
  bool _destroying = false;
  int _display = 0;
  int? _id;
  int? _output;

  int get display => _display;

  final gpuTextureRenderer = FlutterGpuTextureRenderer();

  _GpuTexture();

  create(int d, SessionID sessionId, FFI ffi) async {
    if (support) {
      _sessionId = sessionId;
      _display = d;

      try {
        _id = await gpuTextureRenderer.registerTexture();
        if (_id != null) {
          int id = _id!;
          _textureId = id;
          ffi.textureModel.setGpuTextureId(display: d, id: id);
          final output = await gpuTextureRenderer.output(id);
          _output = output;
          if (output != null) {
            platformFFI.registerGpuTexture(sessionId, d, output);
          }
          debugPrint(
              "create gpu texture: peerId: ${ffi.id} display:$_display, textureId:$id, output:$output");
        }
      } catch (e) {
        debugPrint("Failed to create gpu texture:$e");
      }
    }
  }

  destroy(FFI ffi) async {
    // must stop texture render, render unregistered texture cause crash
    if (!_destroying && support && _sessionId != null && _textureId != -1) {
      _destroying = true;
      platformFFI.registerGpuTexture(_sessionId!, _display, 0);
      // sleep for a while to avoid the texture is used after it's unregistered.
      await Future.delayed(Duration(milliseconds: 100));
      await gpuTextureRenderer.unregisterTexture(_textureId);
      _textureId = -1;
      _destroying = false;
      debugPrint(
          "destroy gpu texture: peerId: ${ffi.id} display:$_display, textureId:$_id, output:$_output");
    }
  }
}

class _Control {
  RxInt textureID = (-1).obs;

  int _rgbaTextureId = -1;
  int get rgbaTextureId => _rgbaTextureId;
  int _gpuTextureId = -1;
  int get gpuTextureId => _gpuTextureId;
  bool _isGpuTexture = false;
  bool get isGpuTexture => _isGpuTexture;

  setTextureType({bool gpuTexture = false}) {
    _isGpuTexture = gpuTexture;
    textureID.value = _isGpuTexture ? gpuTextureId : rgbaTextureId;
  }

  setRgbaTextureId(int id) {
    _rgbaTextureId = id;
    textureID.value = _isGpuTexture ? gpuTextureId : rgbaTextureId;
  }

  setGpuTextureId(int id) {
    _gpuTextureId = id;
    textureID.value = _isGpuTexture ? gpuTextureId : rgbaTextureId;
  }
}

class TextureModel {
  final WeakReference<FFI> parent;
  final Map<int, _Control> _control = {};
  final Map<int, _PixelbufferTexture> _pixelbufferRenderTextures = {};
  final Map<int, _GpuTexture> _gpuRenderTextures = {};

  TextureModel(this.parent);

  setTextureType({required int display, required bool gpuTexture}) {
    debugPrint("setTextureType: display:$display, isGpuTexture:$gpuTexture");
    var texture = _control[display];
    if (texture == null) {
      texture = _Control();
      _control[display] = texture;
    }
    texture.setTextureType(gpuTexture: gpuTexture);
  }

  setRgbaTextureId({required int display, required int id}) {
    var ctl = _control[display];
    if (ctl == null) {
      ctl = _Control();
      _control[display] = ctl;
    }
    ctl.setRgbaTextureId(id);
  }

  setGpuTextureId({required int display, required int id}) {
    var ctl = _control[display];
    if (ctl == null) {
      ctl = _Control();
      _control[display] = ctl;
    }
    ctl.setGpuTextureId(id);
  }

  RxInt getTextureId(int display) {
    var ctl = _control[display];
    if (ctl == null) {
      ctl = _Control();
      _control[display] = ctl;
    }
    return ctl.textureID;
  }

  updateCurrentDisplay(int curDisplay, VoidCallback? callback) async {
    debugPrint(
        "=================================== updateCurrentDisplay: curDisplay:$curDisplay");
    final ffi = parent.target;
    if (ffi == null) return;
    tryCreateTexture(int idx) async {
      if (!_pixelbufferRenderTextures.containsKey(idx)) {
        final renderTexture = _PixelbufferTexture();
        _pixelbufferRenderTextures[idx] = renderTexture;
        await renderTexture.create(idx, ffi.sessionId, ffi);
      }
      if (!_gpuRenderTextures.containsKey(idx)) {
        final renderTexture = _GpuTexture();
        _gpuRenderTextures[idx] = renderTexture;
        await renderTexture.create(idx, ffi.sessionId, ffi);
      }
    }

    tryRemoveTexture(int idx) async {
      _control.remove(idx);
      if (_pixelbufferRenderTextures.containsKey(idx)) {
        await _pixelbufferRenderTextures[idx]!.destroy(true, ffi);
        _pixelbufferRenderTextures.remove(idx);
      }
      if (_gpuRenderTextures.containsKey(idx)) {
        await _gpuRenderTextures[idx]!.destroy(ffi);
        _gpuRenderTextures.remove(idx);
      }
    }

    if (curDisplay == kAllDisplayValue) {
      final displays = ffi.ffiModel.pi.getCurDisplays();
      for (var i = 0; i < displays.length; i++) {
        await tryCreateTexture(i);
      }
    } else {
      await tryCreateTexture(curDisplay);
      for (var i = 0; i < ffi.ffiModel.pi.displays.length; i++) {
        if (i != curDisplay) {
          await tryRemoveTexture(i);
        }
      }
    }
    callback?.call();
  }

  onRemotePageDispose(bool closeSession) async {
    final ffi = parent.target;
    if (ffi == null) return;
    for (final texture in _pixelbufferRenderTextures.values) {
      await texture.destroy(closeSession, ffi);
    }
    for (final texture in _gpuRenderTextures.values) {
      await texture.destroy(ffi);
    }
  }
}
