import 'package:flutter_hbb/common/hbbs/hbbs.dart';
import 'package:flutter_hbb/models/peer_model.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('server memberships survive conversion, cache, and copying', () {
    final payload = PeerPayload.fromJson({
      'id': '123456789',
      'device_group_name': 'Primary',
      'device_group_names': ['Primary', 'Support'],
    });
    final peer = PeerPayload.toPeer(payload);
    expect(peer.deviceGroupNames, ['Primary', 'Support']);
    expect(peer.deviceGroupNames.contains('Support'), isTrue);
    expect(
        Peer.fromJson(peer.toJson()).deviceGroupNames, ['Primary', 'Support']);
    expect(Peer.fromJson(peer.toGroupCacheJson()).deviceGroupNames,
        ['Primary', 'Support']);

    final copy = Peer.copy(peer);
    expect(copy.equal(peer), isTrue);
    copy.device_group_names.remove('Support');
    expect(copy.equal(peer), isFalse);
    expect(peer.deviceGroupNames, ['Primary', 'Support']);
  });

  test('older servers and caches retain their single group', () {
    final payload = PeerPayload.fromJson({
      'id': '123456789',
      'device_group_name': 'Primary',
    });
    final peer = PeerPayload.toPeer(payload);
    expect(peer.deviceGroupNames, ['Primary']);
    expect(
        Peer.fromJson(peer.toGroupCacheJson()).deviceGroupNames, ['Primary']);
    expect(Peer.copy(peer).deviceGroupNames, ['Primary']);
  });

  test(
      'removing a secondary group changes the peer without changing its primary',
      () {
    final before = Peer.fromJson({
      'id': '123456789',
      'device_group_name': 'Primary',
      'device_group_names': ['Primary', 'Support'],
    });
    final after = Peer.fromJson({
      'id': '123456789',
      'device_group_name': 'Primary',
      'device_group_names': ['Primary'],
    });
    expect(before.equal(after), isFalse);
    expect(after.deviceGroupNames.contains('Support'), isFalse);
    expect(after.deviceGroupNames.contains('Primary'), isTrue);
  });

  test('unassigned devices have no device groups', () {
    expect(Peer.fromJson({'id': '123456789'}).deviceGroupNames, isEmpty);
    expect(
        Peer.fromJson({
          'id': '123456789',
          'device_group_names': <String>[],
        }).deviceGroupNames,
        isEmpty);
  });
}
