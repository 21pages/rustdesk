import 'package:flutter_hbb/common/hbbs/hbbs.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('users can be found by either group after a cache round trip', () {
    final user = UserPayload.fromJson({
      'name': 'alice',
      'display_name': 'Alice Smith',
      'group_name': 'Primary',
      'group_names': ['Primary', 'Support'],
    });
    for (final json in [user.toJson(), user.toGroupCacheJson()]) {
      final restored = UserPayload.fromJson(json);
      expect(restored.groupNames, ['Primary', 'Support']);
      expect(restored.matchesSearch('primary'), isTrue);
      expect(restored.matchesSearch('SUPPORT'), isTrue);
      expect(restored.matchesSearch('alice'), isTrue);
      expect(restored.matchesSearch('smith'), isTrue);
      expect(restored.matchesSearch('unrelated'), isFalse);
    }
    final removed = UserPayload.fromJson({
      ...user.toJson(),
      'group_names': ['Primary'],
    });
    expect(removed.matchesSearch('support'), isFalse);
  });

  test('older servers and caches preserve username and single-group search',
      () {
    final user =
        UserPayload.fromJson({'name': 'alice', 'group_name': 'Primary'});
    expect(user.groupNames, ['Primary']);
    expect(user.matchesSearch('primary'), isTrue);
    expect(
        UserPayload.fromJson({'name': 'alice'}).matchesSearch('alice'), isTrue);
    expect(UserPayload.fromJson({'name': 'alice'}).groupNames, isEmpty);
  });
}
