import 'package:flutter/material.dart';

import '../../common.dart';

class ScanPage extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text(translate('Scan QR'))),
      body: Center(child: Text(translate('Unsupported'))),
    );
  }
}
