#import <AVFoundation/AVFoundation.h>
#import <AppKit/AppKit.h>
#import <IOKit/hidsystem/IOHIDLib.h>
#include <IOKit/pwr_mgt/IOPMLib.h>
#include <Security/Authorization.h>
#include <Security/AuthorizationTags.h>


// https://github.com/codebytere/node-mac-permissions/blob/main/permissions.mm

extern "C" bool InputMonitoringAuthStatus(bool prompt) {
    #ifdef NO_InputMonitoringAuthStatus
    return true;
    #else
    if (floor(NSAppKitVersionNumber) >= NSAppKitVersionNumber10_15) {
        IOHIDAccessType theType = IOHIDCheckAccess(kIOHIDRequestTypeListenEvent);
        NSLog(@"IOHIDCheckAccess = %d, kIOHIDAccessTypeGranted = %d", theType, kIOHIDAccessTypeGranted);
        switch (theType) {
            case kIOHIDAccessTypeGranted:
                return true;
                break;
            case kIOHIDAccessTypeDenied: {
                if (prompt) {
                    NSString *urlString = @"x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent";
                    [[NSWorkspace sharedWorkspace] openURL:[NSURL URLWithString:urlString]];
                }
                break;
            }
            case kIOHIDAccessTypeUnknown: {
                if (prompt) {
                    bool result = IOHIDRequestAccess(kIOHIDRequestTypeListenEvent);
                    NSLog(@"IOHIDRequestAccess result = %d", result);
                }
                break;
            }
            default:
                break;
        }
    } else {
        return true;
    }
    return false;
    #endif
}

extern "C" bool Elevate(char* process, char** args) {
    AuthorizationRef authRef;
    OSStatus status;

    status = AuthorizationCreate(NULL, kAuthorizationEmptyEnvironment,
                                kAuthorizationFlagDefaults, &authRef);
    if (status != errAuthorizationSuccess) {
        printf("Failed to create AuthorizationRef\n");
        return false;
    }

    AuthorizationItem authItem = {kAuthorizationRightExecute, 0, NULL, 0};
    AuthorizationRights authRights = {1, &authItem};
    AuthorizationFlags flags = kAuthorizationFlagDefaults |
                                kAuthorizationFlagInteractionAllowed |
                                kAuthorizationFlagPreAuthorize |
                                kAuthorizationFlagExtendRights;
    status = AuthorizationCopyRights(authRef, &authRights, kAuthorizationEmptyEnvironment, flags, NULL);
    if (status != errAuthorizationSuccess) {
        printf("Failed to authorize\n");
        return false;
    }

    if (process != NULL) {
        FILE *pipe = NULL;
        status = AuthorizationExecuteWithPrivileges(authRef, process, kAuthorizationFlagDefaults, args, &pipe);
        if (status != errAuthorizationSuccess) {
            printf("Failed to run as root\n");
            AuthorizationFree(authRef, kAuthorizationFlagDefaults);
            return false;
        }
    }

    AuthorizationFree(authRef, kAuthorizationFlagDefaults);
    return true;
}

extern "C" bool MacCheckAdminAuthorization() {
    return Elevate(NULL, NULL);
}

extern "C" float BackingScaleFactor() {
    NSScreen* s = [NSScreen mainScreen];
    if (s) return [s backingScaleFactor];
    return 1;
}

// https://github.com/jhford/screenresolution/blob/master/cg_utils.c
// https://github.com/jdoupe/screenres/blob/master/setgetscreen.m

size_t bitDepth(CGDisplayModeRef mode) {	
    size_t depth = 0;
    // Deprecated, same display same bpp? 
    // https://stackoverflow.com/questions/8210824/how-to-avoid-cgdisplaymodecopypixelencoding-to-get-bpp
    // https://github.com/libsdl-org/SDL/pull/6628
	CFStringRef pixelEncoding = CGDisplayModeCopyPixelEncoding(mode);	
    // my numerical representation for kIO16BitFloatPixels and kIO32bitFloatPixels	
    // are made up and possibly non-sensical	
    if (kCFCompareEqualTo == CFStringCompare(pixelEncoding, CFSTR(kIO32BitFloatPixels), kCFCompareCaseInsensitive)) {	
        depth = 96;	
    } else if (kCFCompareEqualTo == CFStringCompare(pixelEncoding, CFSTR(kIO64BitDirectPixels), kCFCompareCaseInsensitive)) {	
        depth = 64;	
    } else if (kCFCompareEqualTo == CFStringCompare(pixelEncoding, CFSTR(kIO16BitFloatPixels), kCFCompareCaseInsensitive)) {	
        depth = 48;	
    } else if (kCFCompareEqualTo == CFStringCompare(pixelEncoding, CFSTR(IO32BitDirectPixels), kCFCompareCaseInsensitive)) {	
        depth = 32;	
    } else if (kCFCompareEqualTo == CFStringCompare(pixelEncoding, CFSTR(kIO30BitDirectPixels), kCFCompareCaseInsensitive)) {	
        depth = 30;	
    } else if (kCFCompareEqualTo == CFStringCompare(pixelEncoding, CFSTR(IO16BitDirectPixels), kCFCompareCaseInsensitive)) {	
        depth = 16;	
    } else if (kCFCompareEqualTo == CFStringCompare(pixelEncoding, CFSTR(IO8BitIndexedPixels), kCFCompareCaseInsensitive)) {	
        depth = 8;	
    }	
    CFRelease(pixelEncoding);	
    return depth;	
}

extern "C" bool MacGetModeNum(CGDirectDisplayID display, uint32_t *numModes) {
    CFArrayRef allModes = CGDisplayCopyAllDisplayModes(display, NULL);
    if (allModes == NULL) {
        return false;
    }
    *numModes = CFArrayGetCount(allModes);
    CFRelease(allModes);
    return true;
}

extern "C" bool MacGetModes(CGDirectDisplayID display, uint32_t *widths, uint32_t *heights, uint32_t max, uint32_t *numModes) {
    CGDisplayModeRef currentMode = CGDisplayCopyDisplayMode(display);
    if (currentMode == NULL) {
        return false;
    }
    CFArrayRef allModes = CGDisplayCopyAllDisplayModes(display, NULL);
    if (allModes == NULL) {
        CGDisplayModeRelease(currentMode);
        return false;
    }
    uint32_t allModeCount = CFArrayGetCount(allModes);
    uint32_t realNum = 0;
    for (uint32_t i = 0; i < allModeCount && realNum < max; i++) {
        CGDisplayModeRef mode = (CGDisplayModeRef)CFArrayGetValueAtIndex(allModes, i);
        if (CGDisplayModeGetRefreshRate(currentMode) == CGDisplayModeGetRefreshRate(mode) &&
            bitDepth(currentMode) == bitDepth(mode)) {
            widths[realNum] = (uint32_t)CGDisplayModeGetWidth(mode);
            heights[realNum] = (uint32_t)CGDisplayModeGetHeight(mode);
            realNum++;
        }
    }
    *numModes = realNum;
    CGDisplayModeRelease(currentMode);
    CFRelease(allModes);
    return true;
}

extern "C" bool MacGetMode(CGDirectDisplayID display, uint32_t *width, uint32_t *height) {
    CGDisplayModeRef mode = CGDisplayCopyDisplayMode(display);
    if (mode == NULL) {
        return false;
    }
    *width = (uint32_t)CGDisplayModeGetWidth(mode);
    *height = (uint32_t)CGDisplayModeGetHeight(mode);
    CGDisplayModeRelease(mode);
    return true;
}


static bool setDisplayToMode(CGDirectDisplayID display, CGDisplayModeRef mode) {
    CGError rc;
    CGDisplayConfigRef config;
    rc = CGBeginDisplayConfiguration(&config);
    if (rc != kCGErrorSuccess) {
        return false;
    }
    rc = CGConfigureDisplayWithDisplayMode(config, display, mode, NULL);
    if (rc != kCGErrorSuccess) {
        return false;
    }
    rc = CGCompleteDisplayConfiguration(config, kCGConfigureForSession);
    if (rc != kCGErrorSuccess) {
        return false;
    }
    return true;
}

extern "C" bool MacSetMode(CGDirectDisplayID display, uint32_t width, uint32_t height)
{
    bool ret = false;
    CGDisplayModeRef currentMode = CGDisplayCopyDisplayMode(display);
    if (currentMode == NULL) {
        return ret;
    }
    CFArrayRef allModes = CGDisplayCopyAllDisplayModes(display, NULL);
    if (allModes == NULL) {
        CGDisplayModeRelease(currentMode);
        return ret;
    }
    int numModes = CFArrayGetCount(allModes);
    for (int i = 0; i < numModes; i++) {
        CGDisplayModeRef mode = (CGDisplayModeRef)CFArrayGetValueAtIndex(allModes, i);
        if (width == CGDisplayModeGetWidth(mode) &&
            height == CGDisplayModeGetHeight(mode) && 
            CGDisplayModeGetRefreshRate(currentMode) == CGDisplayModeGetRefreshRate(mode) &&
            bitDepth(currentMode) == bitDepth(mode)) {
            ret = setDisplayToMode(display, mode);
            break;
        }
    }
    CGDisplayModeRelease(currentMode);
    CFRelease(allModes);
    return ret;
}

// https://github.com/videolan/vlc/blob/f7bb59d9f51cc10b25ff86d34a3eff744e60c46e/modules/misc/inhibit/iokit-inhibit.c#L63
struct vlc_inhibit_sys
{
    // Activity IOPMAssertion to wake the display if sleeping
    IOPMAssertionID act_assertion_id;

    // Inhibition IOPMAssertion to keep display or machine from sleeping
    IOPMAssertionID inh_assertion_id;
};
typedef struct vlc_inhibit_sys vlc_inhibit_sys_t;

extern "C" void* MacOpenInhibit() {
    vlc_inhibit_sys_t *sys = (vlc_inhibit_sys_t*)malloc(sizeof(vlc_inhibit_sys_t));
    if (!sys) {
        printf("Failed to malloc vlc_inhibit_sys_t\n");
        return NULL;
    }
    sys->act_assertion_id = kIOPMNullAssertionID;
    sys->inh_assertion_id = kIOPMNullAssertionID;
    return sys;
}

extern "C" bool MacUpdateInhibit(vlc_inhibit_sys_t *sys) {
    if (sys == NULL) return false;
    IOReturn ret;
    CFStringRef activity_reason = CFSTR("RustDesk");
    // Wake up display
    ret = IOPMAssertionDeclareUserActivity(activity_reason,
                                            kIOPMUserActiveLocal,
                                            &(sys->act_assertion_id));
    if (ret != kIOReturnSuccess) {
        printf("Failed to declare user activity (%i)\n", ret);
    }

    // Actual display inhibition assertion
    ret = IOPMAssertionCreateWithName(kIOPMAssertPreventUserIdleDisplaySleep,
                                        kIOPMAssertionLevelOn,
                                        activity_reason,
                                        &(sys->inh_assertion_id));
    if (ret != kIOReturnSuccess) {
        printf("Failed to IOPMAssertionCreateWithName (%i)\n", ret);
    }
    return ret == kIOReturnSuccess;
}

extern "C" void MacCloseInhibit(vlc_inhibit_sys_t *sys) {
    if (sys == NULL) return;
    // Release remaining IOPMAssertion for inhibition, if any
    if (sys->inh_assertion_id != kIOPMNullAssertionID) {
        if (IOPMAssertionRelease(sys->inh_assertion_id) != kIOReturnSuccess) {
            printf("Failed releasing IOPMAssertion on termination\n");
        }
        sys->inh_assertion_id = kIOPMNullAssertionID;
    }

    // Release remaining IOPMAssertion for activity, if any
    if (sys->act_assertion_id != kIOPMNullAssertionID) {
        if (IOPMAssertionRelease(sys->act_assertion_id) != kIOReturnSuccess) {
            printf("Failed releasing IOPMAssertion on termination\n");
        }
        sys->act_assertion_id = kIOPMNullAssertionID;
    }
}