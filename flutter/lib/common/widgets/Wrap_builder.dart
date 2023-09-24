import 'dart:math';

import 'package:flutter/material.dart';

typedef ValueWidgetBuilder<T> = Widget Function(T value);

class WrapBuilder extends StatelessWidget {
  final double itemWidth;
  final List items;
  final ValueWidgetBuilder itemBuilder;

  const WrapBuilder(
      {Key? key,
      required this.itemWidth,
      required this.items,
      required this.itemBuilder})
      : super(key: key);

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(builder: (context, constraints) {
      var cardsPerRow = max(1, constraints.maxWidth ~/ itemWidth);
      return ListView.builder(
        shrinkWrap: true,
        primary: false,
        controller: ScrollController(),
        itemCount: (items.length / cardsPerRow).ceil(),
        itemBuilder: (BuildContext context, int index) {
          var rowItems = items.sublist(cardsPerRow * index,
              min(cardsPerRow * (index + 1), items.length));
          return Row(children: [
            for (final item in rowItems)
              SizedBox(width: itemWidth, child: itemBuilder(item))
          ]);
        },
      );
    });
  }
}
