import 'dart:math';

import 'package:flutter/material.dart';
import 'package:get/get.dart';

typedef ValueWidgetBuilder<T> = Widget Function(T value);

class WrapBuilder extends StatelessWidget {
  final double itemWidth;
  final double itemHeight;
  final double hSpacing;
  final double vSpacing;
  final List items;
  final ValueWidgetBuilder itemBuilder;
  final ScrollController controller = ScrollController();

  WrapBuilder(
      {Key? key,
      required this.itemWidth,
      required this.itemHeight,
      required this.hSpacing,
      required this.vSpacing,
      required this.items,
      required this.itemBuilder})
      : super(key: key);

  @override
  Widget build(BuildContext context) {
    final realWidth = itemWidth + hSpacing;
    final realHeight = itemHeight + vSpacing;
    return LayoutBuilder(builder: (context, constraints) {
      var cardsPerRow = max(1, constraints.maxWidth ~/ realWidth);
      return ListView.builder(
        shrinkWrap: true,
        primary: false,
        itemExtent: realHeight,
        controller: controller,
        itemCount: (items.length / cardsPerRow).ceil(),
        itemBuilder: (BuildContext context, int index) {
          var rowItems = items.sublist(cardsPerRow * index,
              min(cardsPerRow * (index + 1), items.length));
          return Row(children: [
            for (final item in rowItems)
              SizedBox(
                  width: realWidth,
                  height: realHeight,
                  child: itemBuilder(item).marginSymmetric(
                      horizontal: hSpacing / 2, vertical: vSpacing / 2))
          ]);
        },
      );
    });
  }
}
