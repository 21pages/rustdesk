import 'package:flutter_hbb/common.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('without-password links preserve their connection type', () {
    for (final type in [
      'connect',
      'play',
      'file-transfer',
      'view-camera',
      'port-forward',
      'rdp',
      'terminal',
      'terminal-admin',
    ]) {
      expect(
        urlLinkToCmdArgs(
            Uri.parse('rustdesk://$type/123456789?without_password=true')),
        ['--$type', '123456789', '--without-password'],
      );
    }
    expect(
      urlLinkToCmdArgs(Uri.parse('rustdesk://123456789?without_password=true')),
      ['--connect', '123456789', '--without-password'],
    );
  });

  test('omitted and empty passwords retain automatic login', () {
    expect(urlLinkToCmdArgs(Uri.parse('rustdesk://123456789')),
        ['--connect', '123456789']);
    for (final query in ['password=', 'password=&without_password=false']) {
      expect(urlLinkToCmdArgs(Uri.parse('rustdesk://123456789?$query')),
          ['--connect', '123456789', '--password', '']);
    }
  });

  test('without-password remains selected alongside an explicit password', () {
    expect(
      urlLinkToCmdArgs(Uri.parse(
          'rustdesk://123456789?password=provided&without_password=true')),
      [
        '--connect',
        '123456789',
        '--password',
        'provided',
        '--without-password'
      ],
    );
  });
}
