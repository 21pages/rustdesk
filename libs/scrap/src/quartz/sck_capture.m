#import <CoreMedia/CoreMedia.h>
#import <CoreVideo/CoreVideo.h>
#import <Foundation/Foundation.h>
#import <IOSurface/IOSurface.h>
#import <ScreenCaptureKit/ScreenCaptureKit.h>

#include <dlfcn.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

typedef void (*ScrapSckFrameCallback)(void *context, void *surface);
typedef void (*ScrapSckErrorCallback)(void *context, const char *message);

typedef struct {
    int32_t code;
    char message[512];
} ScrapSckError;

static const int64_t kScrapSckTimeoutSeconds = 10;

static void ScrapSckSetError(ScrapSckError *output, NSError *error, NSString *fallback) {
    if (output == NULL) {
        return;
    }
    output->code = error == nil ? -1 : (int32_t)error.code;
    NSString *message = error.localizedDescription ?: fallback ?: @"ScreenCaptureKit failed";
    const char *utf8 = message.UTF8String;
    if (utf8 == NULL) {
        output->message[0] = '\0';
        return;
    }
    strncpy(output->message, utf8, sizeof(output->message) - 1);
    output->message[sizeof(output->message) - 1] = '\0';
}

static NSError *ScrapSckErrorWithMessage(NSString *message) {
    return [NSError errorWithDomain:@"com.rustdesk.scrap.screencapturekit"
                               code:-1
                           userInfo:@{NSLocalizedDescriptionKey : message}];
}

static BOOL ScrapSckWait(dispatch_semaphore_t semaphore) {
    dispatch_time_t timeout = dispatch_time(
        DISPATCH_TIME_NOW, kScrapSckTimeoutSeconds * NSEC_PER_SEC);
    return dispatch_semaphore_wait(semaphore, timeout) == 0;
}

static Class ScrapSckLoadClass(NSString *name) {
    static void *framework = NULL;
    static dispatch_once_t once;
    dispatch_once(&once, ^{
        framework = dlopen(
            "/System/Library/Frameworks/ScreenCaptureKit.framework/ScreenCaptureKit",
            RTLD_LAZY | RTLD_LOCAL);
    });
    if (framework == NULL) {
        return Nil;
    }
    return NSClassFromString(name);
}

static SCShareableContent *ScrapSckGetShareableContent(NSError **outputError) {
    Class contentClass = ScrapSckLoadClass(@"SCShareableContent");
    if (contentClass == Nil) {
        if (outputError != NULL) {
            *outputError = ScrapSckErrorWithMessage(@"ScreenCaptureKit is unavailable");
        }
        return nil;
    }

    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    __block SCShareableContent *content = nil;
    __block NSError *error = nil;
    [(id)contentClass
        getShareableContentExcludingDesktopWindows:NO
                              onScreenWindowsOnly:NO
                                   completionHandler:^(SCShareableContent *value,
                                                       NSError *valueError) {
        content = value;
        error = valueError;
        dispatch_semaphore_signal(semaphore);
    }];

    if (!ScrapSckWait(semaphore)) {
        if (outputError != NULL) {
            *outputError = ScrapSckErrorWithMessage(
                @"Timed out while enumerating shareable screen content");
        }
        return nil;
    }
    if (content == nil && outputError != NULL) {
        *outputError = error ?: ScrapSckErrorWithMessage(
            @"Failed to enumerate shareable screen content");
    }
    return content;
}

static SCDisplay *ScrapSckFindDisplay(SCShareableContent *content,
                                      CGDirectDisplayID displayID) {
    for (SCDisplay *display in content.displays) {
        if (display.displayID == displayID) {
            return display;
        }
    }
    return nil;
}

static NSArray<SCWindow *> *ScrapSckFindWindows(
    SCShareableContent *content,
    const uint32_t *windowIDs,
    size_t windowCount) {
    if (windowIDs == NULL || windowCount == 0) {
        return @[];
    }
    NSMutableSet<NSNumber *> *wanted = [NSMutableSet setWithCapacity:windowCount];
    for (size_t index = 0; index < windowCount; ++index) {
        [wanted addObject:@(windowIDs[index])];
    }
    NSMutableArray<SCWindow *> *windows = [NSMutableArray arrayWithCapacity:wanted.count];
    for (SCWindow *window in content.windows) {
        if ([wanted containsObject:@(window.windowID)]) {
            [windows addObject:window];
        }
    }
    return windows;
}

static NSArray<SCRunningApplication *> *ScrapSckFindOwnApplications(
    SCShareableContent *content) {
    NSString *bundleID = NSBundle.mainBundle.bundleIdentifier;
    NSString *processName = NSProcessInfo.processInfo.processName;
    NSMutableArray<SCRunningApplication *> *applications = [NSMutableArray array];
    for (SCRunningApplication *application in content.applications) {
        BOOL sameBundle = bundleID.length > 0 &&
            [application.bundleIdentifier isEqualToString:bundleID];
        BOOL sameName = processName.length > 0 &&
            [application.applicationName isEqualToString:processName];
        if (sameBundle || sameName) {
            [applications addObject:application];
        }
    }
    return applications;
}

static SCContentFilter *ScrapSckBuildFilter(
    SCShareableContent *content,
    CGDirectDisplayID displayID,
    const uint32_t *windowIDs,
    size_t windowCount,
    BOOL initial,
    NSError **outputError) {
    SCDisplay *display = ScrapSckFindDisplay(content, displayID);
    if (display == nil) {
        if (outputError != NULL) {
            *outputError = ScrapSckErrorWithMessage(@"The requested display is unavailable");
        }
        return nil;
    }

    Class filterClass = ScrapSckLoadClass(@"SCContentFilter");
    if (filterClass == Nil) {
        if (outputError != NULL) {
            *outputError = ScrapSckErrorWithMessage(@"SCContentFilter is unavailable");
        }
        return nil;
    }

    NSArray<SCWindow *> *windows = ScrapSckFindWindows(content, windowIDs, windowCount);
    if (windowCount > 0 && windows.count == windowCount) {
        return [[filterClass alloc] initWithDisplay:display excludingWindows:windows];
    }

    if (initial || windowCount > 0) {
        NSArray<SCRunningApplication *> *applications =
            ScrapSckFindOwnApplications(content);
        return [[filterClass alloc]
            initWithDisplay:display
            excludingApplications:applications
                 exceptingWindows:@[]];
    }

    return [[filterClass alloc] initWithDisplay:display excludingWindows:@[]];
}

@interface ScrapSckCapture : NSObject

@property(nonatomic, strong) SCStream *stream;
@property(nonatomic, strong) dispatch_queue_t queue;
@property(nonatomic, assign) CGDirectDisplayID displayID;
@property(nonatomic, assign) ScrapSckFrameCallback frameCallback;
@property(nonatomic, assign) ScrapSckErrorCallback errorCallback;
@property(nonatomic, assign) void *callbackContext;
@property(nonatomic, assign) BOOL started;

- (BOOL)startWithDisplayID:(CGDirectDisplayID)displayID
                     width:(size_t)width
                    height:(size_t)height
                    cursor:(BOOL)cursor
         excludedWindowIDs:(const uint32_t *)windowIDs
               windowCount:(size_t)windowCount
                     error:(NSError **)outputError;
- (BOOL)updateExcludedWindowIDs:(const uint32_t *)windowIDs
                    windowCount:(size_t)windowCount
                          error:(NSError **)outputError;
- (void)stopSynchronously;

@end

@implementation ScrapSckCapture

- (BOOL)startWithDisplayID:(CGDirectDisplayID)displayID
                     width:(size_t)width
                    height:(size_t)height
                    cursor:(BOOL)cursor
         excludedWindowIDs:(const uint32_t *)windowIDs
               windowCount:(size_t)windowCount
                     error:(NSError **)outputError {
    NSError *error = nil;
    SCShareableContent *content = ScrapSckGetShareableContent(&error);
    if (content == nil) {
        if (outputError != NULL) {
            *outputError = error;
        }
        return NO;
    }
    SCContentFilter *filter = ScrapSckBuildFilter(
        content, displayID, windowIDs, windowCount, YES, &error);
    if (filter == nil) {
        if (outputError != NULL) {
            *outputError = error;
        }
        return NO;
    }

    Class configurationClass = ScrapSckLoadClass(@"SCStreamConfiguration");
    Class streamClass = ScrapSckLoadClass(@"SCStream");
    if (configurationClass == Nil || streamClass == Nil) {
        if (outputError != NULL) {
            *outputError = ScrapSckErrorWithMessage(
                @"ScreenCaptureKit stream classes are unavailable");
        }
        return NO;
    }

    SCStreamConfiguration *configuration = [[configurationClass alloc] init];
    configuration.width = width;
    configuration.height = height;
    configuration.minimumFrameInterval = kCMTimeZero;
    configuration.pixelFormat = kCVPixelFormatType_32BGRA;
    configuration.queueDepth = 3;
    configuration.showsCursor = cursor;
    configuration.scalesToFit = YES;

    self.displayID = displayID;
    self.queue = dispatch_queue_create("com.rustdesk.scrap.screencapturekit",
                                       DISPATCH_QUEUE_SERIAL);
    self.stream = [[streamClass alloc] initWithFilter:filter
                                       configuration:configuration
                                            delegate:(id)self];
    if (![self.stream addStreamOutput:(id)self
                                 type:SCStreamOutputTypeScreen
                   sampleHandlerQueue:self.queue
                                error:&error]) {
        if (outputError != NULL) {
            *outputError = error;
        }
        self.stream = nil;
        self.queue = nil;
        return NO;
    }

    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    __block NSError *startError = nil;
    self.started = YES;
    [self.stream startCaptureWithCompletionHandler:^(NSError *valueError) {
        startError = valueError;
        dispatch_semaphore_signal(semaphore);
    }];
    if (!ScrapSckWait(semaphore)) {
        if (outputError != NULL) {
            *outputError = ScrapSckErrorWithMessage(
                @"Timed out while starting screen capture");
        }
        [self stopSynchronously];
        return NO;
    }
    if (startError != nil) {
        if (outputError != NULL) {
            *outputError = startError;
        }
        [self stopSynchronously];
        return NO;
    }
    return YES;
}

- (BOOL)updateExcludedWindowIDs:(const uint32_t *)windowIDs
                    windowCount:(size_t)windowCount
                          error:(NSError **)outputError {
    NSError *error = nil;
    SCShareableContent *content = ScrapSckGetShareableContent(&error);
    if (content == nil) {
        if (outputError != NULL) {
            *outputError = error;
        }
        return NO;
    }
    SCContentFilter *filter = ScrapSckBuildFilter(
        content, self.displayID, windowIDs, windowCount, NO, &error);
    if (filter == nil) {
        if (outputError != NULL) {
            *outputError = error;
        }
        return NO;
    }

    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    __block NSError *updateError = nil;
    [self.stream updateContentFilter:filter completionHandler:^(NSError *valueError) {
        updateError = valueError;
        dispatch_semaphore_signal(semaphore);
    }];
    if (!ScrapSckWait(semaphore)) {
        if (outputError != NULL) {
            *outputError = ScrapSckErrorWithMessage(
                @"Timed out while updating the screen capture filter");
        }
        return NO;
    }
    if (updateError != nil) {
        if (outputError != NULL) {
            *outputError = updateError;
        }
        return NO;
    }
    return YES;
}

- (void)stopSynchronously {
    if (self.stream == nil) {
        return;
    }
    if (self.started) {
        dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
        [self.stream stopCaptureWithCompletionHandler:^(__unused NSError *error) {
            dispatch_semaphore_signal(semaphore);
        }];
        ScrapSckWait(semaphore);
        self.started = NO;
    }
    NSError *removeError = nil;
    [self.stream removeStreamOutput:(id)self
                               type:SCStreamOutputTypeScreen
                              error:&removeError];
    self.stream = nil;
    self.queue = nil;
}

- (void)stream:(__unused SCStream *)stream
    didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
                   ofType:(SCStreamOutputType)type {
    if (type != SCStreamOutputTypeScreen ||
        !CMSampleBufferIsValid(sampleBuffer) ||
        !CMSampleBufferDataIsReady(sampleBuffer)) {
        return;
    }
    CVPixelBufferRef pixelBuffer = CMSampleBufferGetImageBuffer(sampleBuffer);
    if (pixelBuffer == NULL) {
        return;
    }
    IOSurfaceRef surface = CVPixelBufferGetIOSurface(pixelBuffer);
    if (surface != NULL && self.frameCallback != NULL) {
        self.frameCallback(self.callbackContext, surface);
    }
}

- (void)stream:(__unused SCStream *)stream didStopWithError:(NSError *)error {
    if (self.errorCallback != NULL) {
        self.errorCallback(self.callbackContext, error.localizedDescription.UTF8String);
    }
}

@end

bool scrap_sck_is_available(void) {
    if (@available(macOS 12.3, *)) {
        return ScrapSckLoadClass(@"SCStream") != Nil;
    }
    return false;
}

void *scrap_sck_create(uint32_t displayID,
                       size_t width,
                       size_t height,
                       bool cursor,
                       const uint32_t *windowIDs,
                       size_t windowCount,
                       ScrapSckFrameCallback frameCallback,
                       ScrapSckErrorCallback errorCallback,
                       void *callbackContext,
                       ScrapSckError *outputError) {
    if (!scrap_sck_is_available()) {
        ScrapSckSetError(outputError, nil, @"ScreenCaptureKit is unavailable");
        return NULL;
    }
    @autoreleasepool {
        ScrapSckCapture *capture = [[ScrapSckCapture alloc] init];
        capture.frameCallback = frameCallback;
        capture.errorCallback = errorCallback;
        capture.callbackContext = callbackContext;
        NSError *error = nil;
        if (![capture startWithDisplayID:displayID
                                   width:width
                                  height:height
                                  cursor:cursor
                       excludedWindowIDs:windowIDs
                             windowCount:windowCount
                                   error:&error]) {
            ScrapSckSetError(outputError, error, @"Failed to start screen capture");
            return NULL;
        }
        return (__bridge_retained void *)capture;
    }
}

bool scrap_sck_update_excluded_windows(void *handle,
                                       const uint32_t *windowIDs,
                                       size_t windowCount,
                                       ScrapSckError *outputError) {
    if (handle == NULL) {
        ScrapSckSetError(outputError, nil, @"Screen capture is unavailable");
        return false;
    }
    @autoreleasepool {
        ScrapSckCapture *capture = (__bridge ScrapSckCapture *)handle;
        NSError *error = nil;
        if (![capture updateExcludedWindowIDs:windowIDs
                                  windowCount:windowCount
                                        error:&error]) {
            ScrapSckSetError(outputError, error, @"Failed to update screen capture filter");
            return false;
        }
        return true;
    }
}

void scrap_sck_destroy(void *handle) {
    if (handle == NULL) {
        return;
    }
    @autoreleasepool {
        ScrapSckCapture *capture = (__bridge ScrapSckCapture *)handle;
        capture.frameCallback = NULL;
        capture.errorCallback = NULL;
        [capture stopSynchronously];
        CFBridgingRelease(handle);
    }
}
