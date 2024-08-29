
import { simd } from "wasm-feature-detect";
import '../../ffmpeg/ffmpeg-core';
export async function loadFFmpeg(callback) {
  createFFmpegCore().then((decoder) => {
    callback(decoder);
  });
}