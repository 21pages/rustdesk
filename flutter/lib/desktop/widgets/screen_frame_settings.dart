import 'package:flex_color_picker/flex_color_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/consts.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:get/get.dart';

class ScreenFrameStyleSettings extends StatelessWidget {
  final bool enabled;

  const ScreenFrameStyleSettings({super.key, required this.enabled});

  Color _readColor(String key, Color fallback) {
    final value = bind.mainGetOptionSync(key: key);
    return value.length == 7 && RegExp(r'^#[0-9a-fA-F]{6}$').hasMatch(value)
        ? Color(0xFF000000 | int.parse(value.substring(1), radix: 16))
        : fallback;
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(left: 48, right: 15, bottom: 10),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        _colorPicker(context, 'Viewed screen frame color',
            kOptionScreenFrameColor, const Color(0xFFEF4438)),
        _colorPicker(context, 'Unviewed screen frame color',
            kOptionScreenFrameInactiveColor, const Color(0xFF808080)),
        _slider('Screen frame width', kOptionScreenFrameWidth,
            min: 2, max: 12, fallback: 6, unit: 'px'),
        _slider('Screen frame transparency', kOptionScreenFrameTransparency,
            min: 0, max: 80, fallback: 25, unit: '%'),
      ]),
    );
  }

  Widget _colorPicker(
      BuildContext context, String label, String key, Color fallback) {
    final color = _readColor(key, fallback).obs;
    return Row(children: [
      Expanded(child: Text(translate(label))),
      Obx(() => OutlinedButton(
            onPressed: enabled && !isOptionFixed(key)
                ? () async {
                    final selected = await showColorPickerDialog(
                      context,
                      color.value,
                      pickersEnabled: {
                        ColorPickerType.accent: false,
                        ColorPickerType.wheel: true,
                      },
                      pickerTypeLabels: {
                        ColorPickerType.primary: translate('Primary Color'),
                        ColorPickerType.wheel: translate('HSV Color'),
                      },
                      actionButtons: ColorPickerActionButtons(
                        dialogOkButtonLabel: translate('OK'),
                        dialogCancelButtonLabel: translate('Cancel'),
                      ),
                      showColorCode: true,
                    );
                    if (!context.mounted || selected == color.value) return;
                    await bind.mainSetOption(
                      key: key,
                      value:
                          '#${(selected.value & 0xFFFFFF).toRadixString(16).padLeft(6, '0').toUpperCase()}',
                    );
                    color.value = _readColor(key, fallback);
                  }
                : null,
            child: Row(mainAxisSize: MainAxisSize.min, children: [
              Container(width: 20, height: 20, color: color.value),
              const SizedBox(width: 8),
              Text(
                  '#${(color.value.value & 0xFFFFFF).toRadixString(16).padLeft(6, '0').toUpperCase()}'),
            ]),
          )),
    ]);
  }

  Widget _slider(String label, String key,
      {required int min,
      required int max,
      required int fallback,
      required String unit}) {
    double readValue() {
      final raw = bind.mainGetOptionSync(key: key);
      final parsed =
          RegExp(r'^[+-]?[0-9]+$').hasMatch(raw) ? int.tryParse(raw) : null;
      return (parsed ?? fallback).clamp(min, max).toDouble();
    }

    final value = readValue().obs;
    final editable = enabled && !isOptionFixed(key);
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Text('${translate(label)} ($min–$max $unit)'),
      Obx(() => Row(children: [
            Expanded(
              child: Slider(
                value: value.value,
                min: min.toDouble(),
                max: max.toDouble(),
                divisions: max - min,
                label: '${value.value.round()} $unit',
                onChanged: editable ? (next) => value.value = next : null,
                // Save once on release so dragging does not repeatedly rebuild native windows.
                onChangeEnd: editable
                    ? (next) async {
                        await bind.mainSetOption(
                            key: key, value: next.round().toString());
                        value.value = readValue();
                      }
                    : null,
              ),
            ),
            SizedBox(width: 55, child: Text('${value.value.round()} $unit')),
          ])),
    ]);
  }
}
