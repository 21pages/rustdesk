import 'dart:math';

import 'package:bot_toast/bot_toast.dart';
import 'package:dropdown_button2/dropdown_button2.dart';
import 'package:dynamic_layouts/dynamic_layouts.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hbb/common/formatter/id_formatter.dart';
import 'package:flutter_hbb/common/hbbs/hbbs.dart';
import 'package:flutter_hbb/common/widgets/peer_card.dart';
import 'package:flutter_hbb/common/widgets/peers_view.dart';
import 'package:flutter_hbb/desktop/widgets/popup_menu.dart';
import 'package:flutter_hbb/models/ab_model.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import '../../desktop/widgets/material_mod_popup_menu.dart' as mod_menu;
import 'package:get/get.dart';
import 'package:flex_color_picker/flex_color_picker.dart';

import '../../common.dart';
import 'dialog.dart';
import 'login.dart';

final hideAbTagsPanel = false.obs;

class AddressBook extends StatefulWidget {
  final EdgeInsets? menuPadding;
  const AddressBook({Key? key, this.menuPadding}) : super(key: key);

  @override
  State<StatefulWidget> createState() {
    return _AddressBookState();
  }
}

class _AddressBookState extends State<AddressBook> {
  var menuPos = RelativeRect.fill;

  @override
  void initState() {
    super.initState();
  }

  @override
  Widget build(BuildContext context) => Obx(() {
        if (!gFFI.userModel.isLogin) {
          return Center(
              child: ElevatedButton(
                  onPressed: loginDialog, child: Text(translate("Login"))));
        } else {
          if (gFFI.abModel.currentAbLoading.value &&
              gFFI.abModel.currentAbEmtpy) {
            return const Center(
              child: CircularProgressIndicator(),
            );
          }
          return Column(
            children: [
              buildErrorBanner(context,
                  loading: gFFI.abModel.currentAbLoading,
                  err: gFFI.abModel.currentAbPullError,
                  retry: null,
                  close: () => gFFI.abModel.currentAbPullError.value = ''),
              buildErrorBanner(context,
                  loading: gFFI.abModel.currentAbLoading,
                  err: gFFI.abModel.currentAbPushError,
                  retry: null, // remove retry
                  close: () => gFFI.abModel.currentAbPushError.value = ''),
              Expanded(
                  child: isDesktop
                      ? _buildAddressBookDesktop()
                      : _buildAddressBookMobile())
            ],
          );
        }
      });

  Widget _buildAddressBookDesktop() {
    return Row(
      children: [
        Offstage(
            offstage: hideAbTagsPanel.value,
            child: Container(
              decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(
                      color: Theme.of(context).colorScheme.background)),
              child: Container(
                width: 150,
                height: double.infinity,
                padding: const EdgeInsets.all(8.0),
                child: Column(
                  children: [
                    _buildAbDropdown(),
                    _buildTagHeader().marginOnly(left: 8.0, right: 0),
                    Expanded(
                      child: Container(
                        width: double.infinity,
                        height: double.infinity,
                        child: _buildTags(),
                      ),
                    ),
                    _buildAbPermission(),
                  ],
                ),
              ),
            ).marginOnly(right: 12.0)),
        _buildPeersViews()
      ],
    );
  }

  Widget _buildAddressBookMobile() {
    return Column(
      children: [
        Offstage(
            offstage: hideAbTagsPanel.value,
            child: Container(
              decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(
                      color: Theme.of(context).colorScheme.background)),
              child: Container(
                padding: const EdgeInsets.all(8.0),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    _buildAbDropdown(),
                    _buildTagHeader().marginOnly(left: 8.0, right: 0),
                    Container(
                      width: double.infinity,
                      child: _buildTags(),
                    ),
                    _buildAbPermission(),
                  ],
                ),
              ),
            ).marginOnly(bottom: 12.0)),
        _buildPeersViews()
      ],
    );
  }

  Widget _buildAbPermission() {
    icon(IconData data, String tooltip) {
      return Tooltip(
          message: translate(tooltip),
          waitDuration: Duration.zero,
          child: Icon(data, size: 12.0).marginSymmetric(horizontal: 2.0));
    }

    if (!gFFI.abModel.supportSharedAb.value) return Offstage();
    if (gFFI.abModel.currentName.value == personalAddressBookName) {
      return Row(
        mainAxisAlignment: MainAxisAlignment.end,
        children: [
          icon(Icons.cloud_off, "Personal"),
        ],
      );
    } else {
      final children = [icon(Icons.cloud, "Shared by the team")];
      if (gFFI.abModel.allowedToEditCurrentAb()) {
        children.add(icon(Icons.edit, "You can edit this address book"));
        if (gFFI.abModel.allowedToUpdateCurrentProfile()) {
          children.add(icon(Icons.admin_panel_settings_sharp,
              "You can update permissions or delete this address book"));
        }
      } else {
        children.add(icon(
            Icons.remove_red_eye_outlined, "You can't edit this address book"));
      }
      if (gFFI.abModel.isCurrentAbSharingPassword()) {
        children.add(icon(Icons.key, "This address book shares passwords"));
      } else {
        children.add(
            icon(Icons.key_off, "This address book doesn't share passwords"));
      }
      return Row(
        mainAxisAlignment: MainAxisAlignment.end,
        children: children,
      );
    }
  }

  Widget _buildAbDropdown() {
    if (!gFFI.abModel.supportSharedAb.value) {
      return Offstage();
    }
    final names = gFFI.abModel.addressBookNames();
    final TextEditingController textEditingController = TextEditingController();
    return DropdownButton2<String>(
      value: gFFI.abModel.currentName.value,
      onChanged: (value) {
        if (value != null) {
          gFFI.abModel.setCurrentName(value);
          bind.setLocalFlutterOption(k: 'current-ab-name', v: value);
        }
      },
      items: names
          .map((e) => DropdownMenuItem(
              value: e,
              child: Row(
                children: [
                  Expanded(
                    child: Tooltip(message: e, child: Text(e)),
                  ),
                ],
              )))
          .toList(),
      isExpanded: true,
      dropdownSearchData: DropdownSearchData(
        searchController: textEditingController,
        searchInnerWidgetHeight: 50,
        searchInnerWidget: Container(
          height: 50,
          padding: const EdgeInsets.only(
            top: 8,
            bottom: 4,
            right: 8,
            left: 8,
          ),
          child: TextFormField(
            expands: true,
            maxLines: null,
            controller: textEditingController,
            decoration: InputDecoration(
              isDense: true,
              contentPadding: const EdgeInsets.symmetric(
                horizontal: 10,
                vertical: 8,
              ),
              hintText: translate('Search'),
              hintStyle: const TextStyle(fontSize: 12),
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(8),
              ),
            ),
          ),
        ),
        searchMatchFn: (item, searchValue) {
          return item.value
              .toString()
              .toLowerCase()
              .contains(searchValue.toLowerCase());
        },
      ),
    );
  }

  Widget _buildTagHeader() {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(translate('Tags')),
        Listener(
            onPointerDown: (e) {
              final x = e.position.dx;
              final y = e.position.dy;
              menuPos = RelativeRect.fromLTRB(x, y, x, y);
            },
            onPointerUp: (_) => _showMenu(menuPos),
            child: build_more(context, invert: true)),
      ],
    );
  }

  Widget _buildTags() {
    return Obx(() {
      final List tags;
      if (gFFI.abModel.sortTags.value) {
        tags = gFFI.abModel.currentAbTags.toList();
        tags.sort();
      } else {
        tags = gFFI.abModel.currentAbTags;
      }
      final editPermission = gFFI.abModel.allowedToEditCurrentAb();
      tagBuilder(String e) {
        return AddressBookTag(
            name: e,
            tags: gFFI.abModel.selectedTags,
            onTap: () {
              if (gFFI.abModel.selectedTags.contains(e)) {
                gFFI.abModel.selectedTags.remove(e);
              } else {
                gFFI.abModel.selectedTags.add(e);
              }
            },
            showActionMenu: editPermission);
      }

      final gridView = DynamicGridView.builder(
          shrinkWrap: isMobile,
          gridDelegate: SliverGridDelegateWithWrapping(),
          itemCount: tags.length,
          itemBuilder: (BuildContext context, int index) {
            final e = tags[index];
            return tagBuilder(e);
          });
      final maxHeight = max(MediaQuery.of(context).size.height / 6, 100.0);
      return isDesktop
          ? gridView
          : LimitedBox(maxHeight: maxHeight, child: gridView);
    });
  }

  Widget _buildPeersViews() {
    return Expanded(
      child: Align(
          alignment: Alignment.topLeft,
          child: AddressBookPeersView(
            menuPadding: widget.menuPadding,
            getInitPeers: () => gFFI.abModel.currentAbPeers,
          )),
    );
  }

  @protected
  MenuEntryBase<String> syncMenuItem() {
    return MenuEntrySwitch<String>(
      switchType: SwitchType.scheckbox,
      text: translate('Sync with recent sessions'),
      getter: () async {
        return shouldSyncAb();
      },
      setter: (bool v) async {
        gFFI.abModel.setShouldAsync(v);
      },
      dismissOnClicked: true,
    );
  }

  @protected
  MenuEntryBase<String> sortMenuItem() {
    return MenuEntrySwitch<String>(
      switchType: SwitchType.scheckbox,
      text: translate('Sort tags'),
      getter: () async {
        return shouldSortTags();
      },
      setter: (bool v) async {
        bind.mainSetLocalOption(key: sortAbTagsOption, value: v ? 'Y' : '');
        gFFI.abModel.sortTags.value = v;
      },
      dismissOnClicked: true,
    );
  }

  @protected
  MenuEntryBase<String> filterMenuItem() {
    return MenuEntrySwitch<String>(
      switchType: SwitchType.scheckbox,
      text: translate('Filter by intersection'),
      getter: () async {
        return filterAbTagByIntersection();
      },
      setter: (bool v) async {
        bind.mainSetLocalOption(key: filterAbTagOption, value: v ? 'Y' : '');
        gFFI.abModel.filterByIntersection.value = v;
      },
      dismissOnClicked: true,
    );
  }

  void _showMenu(RelativeRect pos) {
    final currentProfile = gFFI.abModel.current.sharedProfile();
    final shared = [
      getEntry(translate('Add shared address book'),
          () => addOrUpdateSharedAb(null)),
      if (gFFI.abModel.allowedToUpdateCurrentProfile() &&
          currentProfile != null)
        getEntry(translate('Update shared address book'),
            () => addOrUpdateSharedAb(currentProfile)),
      if (gFFI.abModel.addressBooksAllowedToDelete().isNotEmpty)
        getEntry(translate('Delete shared address book'), deleteSharedAb),
      MenuEntryDivider<String>(),
    ];
    final editPermission = gFFI.abModel.allowedToEditCurrentAb();
    final items = [
      if (gFFI.abModel.supportSharedAb.value) ...shared,
      if (editPermission) getEntry(translate("Add ID"), addIdToCurrentAb),
      if (editPermission) getEntry(translate("Add Tag"), abAddTag),
      getEntry(translate("Unselect all tags"), gFFI.abModel.unsetSelectedTags),
      sortMenuItem(),
      syncMenuItem(),
      filterMenuItem(),
    ];

    mod_menu.showMenu(
      context: context,
      position: pos,
      items: items
          .map((e) => e.build(
              context,
              MenuConfig(
                  commonColor: CustomPopupMenuTheme.commonColor,
                  height: CustomPopupMenuTheme.height,
                  dividerHeight: CustomPopupMenuTheme.dividerHeight)))
          .expand((i) => i)
          .toList(),
      elevation: 8,
    );
  }

  void addIdToCurrentAb() async {
    if (gFFI.abModel.isCurrentAbFull(true)) {
      return;
    }
    var isInProgress = false;
    IDTextEditingController idController = IDTextEditingController(text: '');
    TextEditingController aliasController = TextEditingController(text: '');
    TextEditingController passwordController = TextEditingController(text: '');
    final tags = List.of(gFFI.abModel.currentAbTags);
    var selectedTag = List<dynamic>.empty(growable: true).obs;
    final style = TextStyle(fontSize: 14.0);
    String? errorMsg;
    final passwordShared = gFFI.abModel.isCurrentAbSharingPassword();

    gFFI.dialogManager.show((setState, close, context) {
      submit() async {
        setState(() {
          isInProgress = true;
          errorMsg = null;
        });
        String id = idController.id;
        if (id.isEmpty) {
          // pass
        } else {
          if (gFFI.abModel.idContainByCurrent(id)) {
            setState(() {
              isInProgress = false;
              errorMsg = translate('ID already exists');
            });
            return;
          }
          var password = '';
          if (passwordShared) {
            password = passwordController.text;
          }
          await gFFI.abModel.addIdToCurrent(
              id, aliasController.text.trim(), password, selectedTag);
          this.setState(() {});
          // final currentPeers
        }
        close();
      }

      double marginBottom = 4;
      return CustomAlertDialog(
        title: Text(translate("Add ID")),
        content: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Column(
              children: [
                Align(
                  alignment: Alignment.centerLeft,
                  child: Row(
                    children: [
                      Text(
                        '*',
                        style: TextStyle(color: Colors.red, fontSize: 14),
                      ),
                      Text(
                        'ID',
                        style: style,
                      ),
                    ],
                  ),
                ).marginOnly(bottom: marginBottom),
                TextField(
                  controller: idController,
                  inputFormatters: [IDTextInputFormatter()],
                  decoration: InputDecoration(errorText: errorMsg),
                ),
                Align(
                  alignment: Alignment.centerLeft,
                  child: Text(
                    translate('Alias'),
                    style: style,
                  ),
                ).marginOnly(top: 8, bottom: marginBottom),
                TextField(
                  controller: aliasController,
                ),
                if (passwordShared)
                  Align(
                    alignment: Alignment.centerLeft,
                    child: Text(
                      translate('Password'),
                      style: style,
                    ),
                  ).marginOnly(top: 8, bottom: marginBottom),
                if (passwordShared)
                  TextField(
                    controller: passwordController,
                    obscureText: true,
                  ),
                Align(
                  alignment: Alignment.centerLeft,
                  child: Text(
                    translate('Tags'),
                    style: style,
                  ),
                ).marginOnly(top: 8, bottom: marginBottom),
                Align(
                  alignment: Alignment.centerLeft,
                  child: Wrap(
                    children: tags
                        .map((e) => AddressBookTag(
                            name: e,
                            tags: selectedTag,
                            onTap: () {
                              if (selectedTag.contains(e)) {
                                selectedTag.remove(e);
                              } else {
                                selectedTag.add(e);
                              }
                            },
                            showActionMenu: false))
                        .toList(growable: false),
                  ),
                ),
              ],
            ),
            const SizedBox(
              height: 4.0,
            ),
            // NOT use Offstage to wrap LinearProgressIndicator
            if (isInProgress) const LinearProgressIndicator(),
          ],
        ),
        actions: [
          dialogButton("Cancel", onPressed: close, isOutline: true),
          dialogButton("OK", onPressed: submit),
        ],
        onSubmit: submit,
        onCancel: close,
      );
    });
  }

  void abAddTag() async {
    var field = "";
    var msg = "";
    var isInProgress = false;
    TextEditingController controller = TextEditingController(text: field);
    gFFI.dialogManager.show((setState, close, context) {
      submit() async {
        setState(() {
          msg = "";
          isInProgress = true;
        });
        field = controller.text.trim();
        if (field.isEmpty) {
          // pass
        } else {
          final tags = field.trim().split(RegExp(r"[\s,;\n]+"));
          field = tags.join(',');
          gFFI.abModel.addTags(tags);
          // final currentPeers
        }
        close();
      }

      return CustomAlertDialog(
        title: Text(translate("Add Tag")),
        content: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(translate("whitelist_sep")),
            const SizedBox(
              height: 8.0,
            ),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    maxLines: null,
                    decoration: InputDecoration(
                      errorText: msg.isEmpty ? null : translate(msg),
                    ),
                    controller: controller,
                    autofocus: true,
                  ),
                ),
              ],
            ),
            const SizedBox(
              height: 4.0,
            ),
            // NOT use Offstage to wrap LinearProgressIndicator
            if (isInProgress) const LinearProgressIndicator(),
          ],
        ),
        actions: [
          dialogButton("Cancel", onPressed: close, isOutline: true),
          dialogButton("OK", onPressed: submit),
        ],
        onSubmit: submit,
        onCancel: close,
      );
    });
  }

  void addOrUpdateSharedAb(SharedAbProfile? profile) async {
    final isAdd = profile == null;
    var msg = "";
    var isInProgress = false;
    final style = TextStyle(fontSize: 14.0);
    double marginBottom = 4;
    TextEditingController nameController =
        TextEditingController(text: profile?.name ?? '');
    TextEditingController noteController =
        TextEditingController(text: profile?.note ?? '');
    RxBool editPermission = (profile?.edit ?? false).obs;
    RxBool pwdPermission = (profile?.pwd ?? false).obs;

    gFFI.dialogManager.show((setState, close, context) {
      submit() async {
        final name = nameController.text.trim();
        if (isAdd && name.isEmpty) {
          // pass
        } else {
          final note = noteController.text.trim();
          setState(() {
            msg = "";
            isInProgress = true;
          });
          final errMsg = (profile == null
              ? await gFFI.abModel.addSharedAb(
                  name, note, editPermission.value, pwdPermission.value)
              : await gFFI.abModel.updateSharedAb(profile.guid, name, note,
                  editPermission.value, pwdPermission.value));
          if (errMsg.isNotEmpty) {
            setState(() {
              msg = errMsg;
              isInProgress = false;
            });
            return;
          }
          await gFFI.abModel.pullAb();
          if (gFFI.abModel.addressBookNames().contains(name)) {
            gFFI.abModel.setCurrentName(name);
          }
        }
        close();
      }

      return CustomAlertDialog(
        title:
            Text(translate("${isAdd ? 'Add' : 'Update'} shared address book")),
        content: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Align(
              alignment: Alignment.centerLeft,
              child: Row(
                children: [
                  Text(
                    '*',
                    style: TextStyle(color: Colors.red, fontSize: 14),
                  ),
                  Text(
                    translate('Name'),
                    style: style,
                  ),
                ],
              ),
            ).marginOnly(bottom: marginBottom),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    maxLines: null,
                    decoration: InputDecoration(
                      errorText: msg.isEmpty ? null : translate(msg),
                      errorMaxLines: 3,
                    ),
                    controller: nameController,
                    autofocus: true,
                  ),
                ),
              ],
            ),
            const SizedBox(
              height: 4.0,
            ),
            Align(
              alignment: Alignment.centerLeft,
              child: Text(
                translate('Note'),
                style: style,
              ),
            ).marginOnly(top: 8, bottom: marginBottom),
            TextField(
              controller: noteController,
              maxLength: 100,
            ),
            const SizedBox(
              height: 4.0,
            ),
            Align(
              alignment: Alignment.centerLeft,
              child: Text(
                translate('Allow others to edit (excluding admin)'),
                style: style,
              ),
            ).marginOnly(top: 8, bottom: marginBottom),
            Obx(() => Switch(
                value: editPermission.value,
                onChanged: (v) {
                  editPermission.value = v;
                })),
            const SizedBox(
              height: 4.0,
            ),
            Align(
              alignment: Alignment.centerLeft,
              child: Text(
                translate('Sharing passwords within the team'),
                style: style,
              ),
            ).marginOnly(top: 8, bottom: marginBottom),
            Obx(() => Switch(
                value: pwdPermission.value,
                onChanged: (v) {
                  pwdPermission.value = v;
                })),
            const SizedBox(
              height: 4.0,
            ),
            // NOT use Offstage to wrap LinearProgressIndicator
            if (isInProgress) const LinearProgressIndicator(),
          ],
        ),
        actions: [
          dialogButton("Cancel", onPressed: close, isOutline: true),
          dialogButton("OK", onPressed: submit),
        ],
        onSubmit: submit,
        onCancel: close,
      );
    });
  }

  void deleteSharedAb() async {
    RxBool isInProgress = false.obs;
    final names = gFFI.abModel.addressBooksAllowedToDelete();
    if (names.isEmpty) return;
    RxString currentName = gFFI.abModel.currentName.value.obs;
    if (!names.contains(currentName.value)) {
      currentName.value = names[0];
    }
    gFFI.dialogManager.show((setState, close, context) {
      submit() async {
        isInProgress.value = true;
        String errMsg = await gFFI.abModel.deleteSharedAb(currentName.value);
        close();
        isInProgress.value = false;
        if (errMsg.isEmpty) {
          showToast(translate('Successful'));
        } else {
          BotToast.showText(contentColor: Colors.red, text: translate(errMsg));
        }
        gFFI.abModel.pullAb();
      }

      cancel() {
        close();
      }

      return CustomAlertDialog(
        title: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(IconFont.addressBook, color: MyTheme.accent),
            Text(translate('Delete shared address book')).paddingOnly(left: 10),
          ],
        ),
        content: Obx(() => Column(
              crossAxisAlignment: CrossAxisAlignment.center,
              children: [
                DropdownMenu(
                  initialSelection: currentName.value,
                  onSelected: (value) {
                    if (value != null) {
                      currentName.value = value;
                    }
                  },
                  dropdownMenuEntries: names
                      .map((e) => DropdownMenuEntry(value: e, label: e))
                      .toList(),
                  inputDecorationTheme: InputDecorationTheme(
                      isDense: true, border: UnderlineInputBorder()),
                ),
                // NOT use Offstage to wrap LinearProgressIndicator
                isInProgress.value
                    ? const LinearProgressIndicator()
                    : Offstage()
              ],
            )),
        actions: [
          dialogButton(
            "Cancel",
            icon: Icon(Icons.close_rounded),
            onPressed: cancel,
            isOutline: true,
          ),
          dialogButton(
            "OK",
            icon: Icon(Icons.done_rounded),
            onPressed: submit,
          ),
        ],
        onSubmit: submit,
        onCancel: cancel,
      );
    });
  }
}

class AddressBookTag extends StatelessWidget {
  final String name;
  final RxList<dynamic> tags;
  final Function()? onTap;
  final bool showActionMenu;

  const AddressBookTag(
      {Key? key,
      required this.name,
      required this.tags,
      this.onTap,
      this.showActionMenu = true})
      : super(key: key);

  @override
  Widget build(BuildContext context) {
    var pos = RelativeRect.fill;

    void setPosition(TapDownDetails e) {
      final x = e.globalPosition.dx;
      final y = e.globalPosition.dy;
      pos = RelativeRect.fromLTRB(x, y, x, y);
    }

    const double radius = 8;
    return GestureDetector(
      onTap: onTap,
      onTapDown: showActionMenu ? setPosition : null,
      onSecondaryTapDown: showActionMenu ? setPosition : null,
      onSecondaryTap: showActionMenu ? () => _showMenu(context, pos) : null,
      onLongPress: showActionMenu ? () => _showMenu(context, pos) : null,
      child: Obx(() => Container(
            decoration: BoxDecoration(
                color: tags.contains(name)
                    ? gFFI.abModel.getCurrentAbTagColor(name)
                    : Theme.of(context).colorScheme.background,
                borderRadius: BorderRadius.circular(4)),
            margin: const EdgeInsets.symmetric(horizontal: 4.0, vertical: 4.0),
            padding: const EdgeInsets.symmetric(vertical: 2.0, horizontal: 6.0),
            child: IntrinsicWidth(
              child: Row(
                children: [
                  Container(
                    width: radius,
                    height: radius,
                    decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: tags.contains(name)
                            ? Colors.white
                            : gFFI.abModel.getCurrentAbTagColor(name)),
                  ).marginOnly(right: radius / 2),
                  Expanded(
                    child: Text(name,
                        style: TextStyle(
                            overflow: TextOverflow.ellipsis,
                            color: tags.contains(name) ? Colors.white : null)),
                  ),
                ],
              ),
            ),
          )),
    );
  }

  void _showMenu(BuildContext context, RelativeRect pos) {
    final items = [
      getEntry(translate("Rename"), () {
        renameDialog(
            oldName: name,
            validator: (String? newName) {
              if (newName == null || newName.isEmpty) {
                return translate('Can not be empty');
              }
              if (newName != name &&
                  gFFI.abModel.currentAbTags.contains(newName)) {
                return translate('Already exists');
              }
              return null;
            },
            onSubmit: (String newName) {
              if (name != newName) {
                gFFI.abModel.renameTag(name, newName);
              }
              Future.delayed(Duration.zero, () => Get.back());
            },
            onCancel: () {
              Future.delayed(Duration.zero, () => Get.back());
            });
      }),
      getEntry(translate(translate('Change Color')), () async {
        final model = gFFI.abModel;
        Color oldColor = model.getCurrentAbTagColor(name);
        Color newColor = await showColorPickerDialog(
          context,
          oldColor,
          pickersEnabled: {
            ColorPickerType.accent: false,
            ColorPickerType.wheel: true,
          },
          pickerTypeLabels: {
            ColorPickerType.primary: translate("Primary Color"),
            ColorPickerType.wheel: translate("HSV Color"),
          },
          actionButtons: ColorPickerActionButtons(
              dialogOkButtonLabel: translate("OK"),
              dialogCancelButtonLabel: translate("Cancel")),
          showColorCode: true,
        );
        if (oldColor != newColor) {
          model.setTagColor(name, newColor);
        }
      }),
      getEntry(translate("Delete"), () {
        gFFI.abModel.deleteTag(name);
        Future.delayed(Duration.zero, () => Get.back());
      }),
    ];

    mod_menu.showMenu(
      context: context,
      position: pos,
      items: items
          .map((e) => e.build(
              context,
              MenuConfig(
                  commonColor: CustomPopupMenuTheme.commonColor,
                  height: CustomPopupMenuTheme.height,
                  dividerHeight: CustomPopupMenuTheme.dividerHeight)))
          .expand((i) => i)
          .toList(),
      elevation: 8,
    );
  }
}

MenuEntryButton<String> getEntry(String title, VoidCallback proc) {
  return MenuEntryButton<String>(
    childBuilder: (TextStyle? style) => Text(
      title,
      style: style,
    ),
    proc: proc,
    dismissOnClicked: true,
  );
}
