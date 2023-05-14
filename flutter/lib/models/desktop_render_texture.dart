import 'package:flutter/material.dart';
import 'package:flutter_gpu_texture_renderer/flutter_gpu_texture_renderer.dart';
import 'package:flutter_hbb/models/model.dart';
import 'package:texture_rgba_renderer/texture_rgba_renderer.dart';

import '../../common.dart';
import './platform_model.dart';

class PixelbufferRenderTexture {
  int _textureKey = -1;
  SessionID? _sessionId;
  final support = bind.mainHasPixelbufferTextureRender();
  bool _destroying = false;

  final textureRenderer = TextureRgbaRenderer();

  PixelbufferRenderTexture();

  create(SessionID sessionId, FFI ffi) {
    if (support) {
      _textureKey = bind.getNextTextureKey();
      _sessionId = sessionId;

      textureRenderer.createTexture(_textureKey).then((id) async {
        if (id != -1) {
          ffi.imageModel.setRgbaTextureId(id);
          final ptr = await textureRenderer.getTexturePtr(_textureKey);
          platformFFI.registerPixelbufferTexture(sessionId, ptr);
        }
      });
    }
  }

  destroy(bool unregisterTexture) async {
    if (!_destroying && support && _textureKey != -1 && _sessionId != null) {
      _destroying = true;
      if (unregisterTexture) {
        platformFFI.registerPixelbufferTexture(_sessionId!, 0);
        // sleep for a while to avoid the texture is used after it's unregistered.
        await Future.delayed(Duration(milliseconds: 100));
      }
      await textureRenderer.closeTexture(_textureKey);
      _textureKey = -1;
    }
  }
}

class GpuRenderTexture {
  int _textureId = -1;
  SessionID? _sessionId;
  final support = bind.mainHasGpuTextureRender();
  bool _destroying = false;

  final gpuTextureRenderer = FlutterGpuTextureRenderer();

  GpuRenderTexture();

  create(SessionID sessionId, FFI ffi) {
    if (support) {
      _sessionId = sessionId;

      gpuTextureRenderer.registerTexture().then((id) async {
        debugPrint("gpu texture id: $id");
        if (id != null) {
          _textureId = id;
          ffi.imageModel.setGpuTextureId(id);
          final output = await gpuTextureRenderer.output(id);
          if (output != null) {
            platformFFI.registerGpuTexture(sessionId, output);
          }
        }
      }, onError: (err) {
        debugPrint("Failed to register gpu texture:$err");
      });
    }
  }

  destroy(bool unregisterTexture) async {
    if (!_destroying && support && _sessionId != null && _textureId != -1) {
      _destroying = true;
      if (unregisterTexture) {
        platformFFI.registerGpuTexture(_sessionId!, 0);
        // sleep for a while to avoid the texture is used after it's unregistered.
        await Future.delayed(Duration(milliseconds: 100));
      }
      await gpuTextureRenderer.unregisterTexture(_textureId);
      _textureId = -1;
      _destroying = false;
    }
  }
}
