import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/models/server_model.dart';

import 'cm_demo.dart' as cm_demo;

void main() {
  test('connection manager demo clients match the current Client API', () {
    expect(cm_demo.testClients, hasLength(4));
    expect(cm_demo.testClients.map((client) => client.name), [
      'UserAAAAAA',
      'UserBBBBB',
      'UserC',
      'UserDDDDDDDDDDDd',
    ]);
    expect(
      cm_demo.testClients.every(
          (client) => client.keyboard && !client.clipboard && !client.audio),
      isTrue,
    );
  });

  test('connection start time survives client state serialization', () {
    final client = Client.fromJson({
      'id': 1,
      'authorized': true,
      'connected_at': 123456789,
      'is_file_transfer': false,
      'is_view_camera': false,
      'is_terminal': false,
      'port_forward': '',
      'name': 'Remote user',
      'avatar': '',
      'peer_id': '123456789',
      'keyboard': true,
      'clipboard': true,
      'audio': true,
      'file': true,
      'restart': true,
      'recording': false,
      'block_input': false,
      'privacy_mode': false,
      'disconnected': false,
      'from_switch': false,
      'in_voice_call': false,
      'incoming_voice_call': false,
    });

    expect(client.connectedAt, 123456789);
    expect(client.toJson()['connected_at'], 123456789);
  });
}
