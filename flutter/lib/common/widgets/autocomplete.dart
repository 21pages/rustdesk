import 'package:flutter/material.dart';
import 'package:flutter_hbb/common/formatter/id_formatter.dart';
import '../../../models/platform_model.dart';
import 'package:flutter_hbb/models/peer_model.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/common/widgets/peer_card.dart';

class AllPeersLoader {
  static const _cbQueryOnlines = 'callback_query_onlines';
  static const _dropdownQueryInterval = Duration(seconds: 3);

  List<Peer> peers = [];

  bool _isPeersLoading = false;
  bool _isPeersLoaded = false;

  final String _listenerKey = 'AllPeersLoader';
  var _lastDropdownQueryIds = <String>{};
  var _lastDropdownQueryTime = DateTime.fromMillisecondsSinceEpoch(0);

  late void Function(VoidCallback) setState;

  bool get needLoad => !_isPeersLoaded && !_isPeersLoading;
  bool get isPeersLoaded => _isPeersLoaded;

  AllPeersLoader();

  void init(void Function(VoidCallback) setState) {
    this.setState = setState;
    gFFI.recentPeersModel.addListener(_mergeAllPeers);
    gFFI.lanPeersModel.addListener(_mergeAllPeers);
    gFFI.abModel.addPeerUpdateListener(_listenerKey, _mergeAllPeers);
    gFFI.groupModel.addPeerUpdateListener(_listenerKey, _mergeAllPeers);
    platformFFI.registerEventHandler(_cbQueryOnlines, _listenerKey,
        (evt) async {
      _updateDropdownOnlineState(evt);
    }, replace: true);
  }

  void clear() {
    gFFI.recentPeersModel.removeListener(_mergeAllPeers);
    gFFI.lanPeersModel.removeListener(_mergeAllPeers);
    gFFI.abModel.removePeerUpdateListener(_listenerKey);
    gFFI.groupModel.removePeerUpdateListener(_listenerKey);
    platformFFI.unregisterEventHandler(_cbQueryOnlines, _listenerKey);
  }

  Future<void> getAllPeers() async {
    if (!needLoad) {
      return;
    }
    _isPeersLoading = true;

    if (gFFI.recentPeersModel.peers.isEmpty) {
      bind.mainLoadRecentPeers();
    }
    if (gFFI.lanPeersModel.peers.isEmpty) {
      bind.mainLoadLanPeers();
    }
    // No need to care about peers from abModel, and group model.
    // Because they will pull data in `refreshCurrentUser()` on startup.

    final startTime = DateTime.now();
    _mergeAllPeers();
    final diffTime = DateTime.now().difference(startTime).inMilliseconds;
    if (diffTime < 100) {
      await Future.delayed(Duration(milliseconds: diffTime));
    }
  }

  void queryDropdownOnlineStatus(Iterable<Peer> dropdownPeers) {
    final ids = dropdownPeers
        .map((peer) => peer.id)
        .where((id) => id.isNotEmpty)
        .toSet();
    if (ids.isEmpty) {
      _lastDropdownQueryIds = {};
      return;
    }

    final now = DateTime.now();
    if (_setEquals(ids, _lastDropdownQueryIds) &&
        now.difference(_lastDropdownQueryTime) < _dropdownQueryInterval) {
      return;
    }

    _lastDropdownQueryIds = ids;
    _lastDropdownQueryTime = now;
    bind.queryOnlines(ids: ids.toList(growable: false));
  }

  bool _setEquals(Set<String> a, Set<String> b) {
    return a.length == b.length && a.every(b.contains);
  }

  Set<String> _splitIds(dynamic ids) {
    if (ids is! String || ids.isEmpty) {
      return {};
    }
    return ids.split(',').where((id) => id.isNotEmpty).toSet();
  }

  void _updateDropdownOnlineState(Map<String, dynamic> evt) {
    final onlines = _splitIds(evt['onlines']);
    final offlines = _splitIds(evt['offlines']);
    if (onlines.isEmpty && offlines.isEmpty) {
      return;
    }

    var changed = false;
    for (final peer in peers) {
      if (onlines.contains(peer.id) && !peer.online) {
        peer.online = true;
        changed = true;
      } else if (offlines.contains(peer.id) && peer.online) {
        peer.online = false;
        changed = true;
      }
    }
    if (changed) {
      setState(() {});
    }
  }

  void _mergeAllPeers() {
    final combinedPeers = <String, Peer>{};
    for (var p in gFFI.abModel.allPeers()) {
      if (!combinedPeers.containsKey(p.id)) {
        combinedPeers[p.id] = p;
      }
    }
    for (var p in gFFI.groupModel.peers.map((e) => Peer.copy(e)).toList()) {
      if (!combinedPeers.containsKey(p.id)) {
        combinedPeers[p.id] = p;
      }
    }

    List<Peer> parsedPeers = combinedPeers.values.toList();

    Set<String> peerIds = combinedPeers.keys.toSet();
    for (final peer in gFFI.lanPeersModel.peers) {
      if (!peerIds.contains(peer.id)) {
        parsedPeers.add(peer);
        peerIds.add(peer.id);
      } else if (peer.online) {
        combinedPeers[peer.id]?.online = true;
      }
    }

    for (final peer in gFFI.recentPeersModel.peers) {
      if (!peerIds.contains(peer.id)) {
        parsedPeers.add(peer);
        peerIds.add(peer.id);
      } else if (peer.online) {
        combinedPeers[peer.id]?.online = true;
      }
    }
    for (final id in gFFI.recentPeersModel.restPeerIds) {
      if (!peerIds.contains(id)) {
        parsedPeers.add(Peer.fromJson({'id': id}));
        peerIds.add(id);
      }
    }

    peers = parsedPeers;
    setState(() {
      _isPeersLoading = false;
      _isPeersLoaded = true;
    });
  }
}

class AutocompletePeerTile extends StatefulWidget {
  final VoidCallback onSelect;
  final Peer peer;

  const AutocompletePeerTile({
    Key? key,
    required this.onSelect,
    required this.peer,
  }) : super(key: key);

  @override
  AutocompletePeerTileState createState() => AutocompletePeerTileState();
}

class AutocompletePeerTileState extends State<AutocompletePeerTile> {
  List _frontN<T>(List list, int n) {
    if (list.length <= n) {
      return list;
    } else {
      return list.sublist(0, n);
    }
  }

  @override
  Widget build(BuildContext context) {
    final double tileRadius = 5;
    final name =
        '${widget.peer.username}${widget.peer.username.isNotEmpty && widget.peer.hostname.isNotEmpty ? '@' : ''}${widget.peer.hostname}';
    final greyStyle = TextStyle(
        fontSize: 11,
        color: Theme.of(context).textTheme.titleLarge?.color?.withOpacity(0.6));
    final child = GestureDetector(
        onTap: () => widget.onSelect(),
        child: Padding(
            padding: EdgeInsets.only(left: 5, right: 5),
            child: Container(
                height: 42,
                margin: EdgeInsets.only(bottom: 5),
                child: Row(
                  mainAxisSize: MainAxisSize.max,
                  children: [
                    Container(
                        decoration: BoxDecoration(
                          color: str2color(
                              '${widget.peer.id}${widget.peer.platform}', 0x7f),
                          borderRadius: BorderRadius.only(
                            topLeft: Radius.circular(tileRadius),
                            bottomLeft: Radius.circular(tileRadius),
                          ),
                        ),
                        alignment: Alignment.center,
                        width: 42,
                        height: null,
                        child: Padding(
                            padding: EdgeInsets.all(6),
                            child: getPlatformImage(widget.peer.platform,
                                size: 30))),
                    Expanded(
                      child: Container(
                          padding: EdgeInsets.only(left: 10),
                          decoration: BoxDecoration(
                            color: Theme.of(context).colorScheme.background,
                            borderRadius: BorderRadius.only(
                              topRight: Radius.circular(tileRadius),
                              bottomRight: Radius.circular(tileRadius),
                            ),
                          ),
                          child: Row(
                            children: [
                              Expanded(
                                  child: Container(
                                      margin: EdgeInsets.only(top: 2),
                                      child: Container(
                                          margin: EdgeInsets.only(top: 2),
                                          child: Column(
                                            children: [
                                              Container(
                                                  margin:
                                                      EdgeInsets.only(top: 2),
                                                  child: Row(children: [
                                                    getOnline(
                                                        8, widget.peer.online),
                                                    Expanded(
                                                        child: Text(
                                                      widget.peer.alias.isEmpty
                                                          ? formatID(
                                                              widget.peer.id)
                                                          : widget.peer.alias,
                                                      overflow:
                                                          TextOverflow.ellipsis,
                                                      style: Theme.of(context)
                                                          .textTheme
                                                          .titleSmall,
                                                    )),
                                                    widget.peer.alias.isNotEmpty
                                                        ? Padding(
                                                            padding:
                                                                const EdgeInsets
                                                                    .only(
                                                                    left: 5,
                                                                    right: 5),
                                                            child: Text(
                                                              "(${widget.peer.id})",
                                                              style: greyStyle,
                                                              overflow:
                                                                  TextOverflow
                                                                      .ellipsis,
                                                            ))
                                                        : Container(),
                                                  ])),
                                              Align(
                                                alignment: Alignment.centerLeft,
                                                child: Text(
                                                  name,
                                                  style: greyStyle,
                                                  textAlign: TextAlign.start,
                                                  overflow:
                                                      TextOverflow.ellipsis,
                                                ),
                                              ),
                                            ],
                                          )))),
                            ],
                          )),
                    )
                  ],
                ))));
    final colors = _frontN(widget.peer.tags, 25)
        .map((e) => gFFI.abModel.getCurrentAbTagColor(e))
        .toList();
    return Tooltip(
      message: !(isDesktop || isWebDesktop)
          ? ''
          : widget.peer.tags.isNotEmpty
              ? '${translate('Tags')}: ${widget.peer.tags.join(', ')}'
              : '',
      child: Stack(children: [
        child,
        if (colors.isNotEmpty)
          Positioned(
            top: 5,
            right: 10,
            child: CustomPaint(
              painter: TagPainter(radius: 3, colors: colors),
            ),
          )
      ]),
    );
  }
}
