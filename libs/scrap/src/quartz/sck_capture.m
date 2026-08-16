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

typedef enum {
    ScrapSckUpdateIdle = 0,
    ScrapSckUpdatePending = 1,
    ScrapSckUpdateApplied = 2,
    ScrapSckUpdateNotReady = 3,
    ScrapSckUpdateFailed = 4,
} ScrapSckUpdateResult;

static const int64_t kScrapSckTimeoutSeconds = 10;
static const NSInteger kScrapSckWindowsNotReadyErrorCode = 1;
static NSString *const kScrapSckErrorDomain = @"com.rustdesk.scrap.screencapturekit";
static char kScrapSckCaptureQueueKey;
static char kScrapSckFilterQueueKey;

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
    return [NSError errorWithDomain:kScrapSckErrorDomain
                               code:-1
                           userInfo:@{NSLocalizedDescriptionKey : message}];
}

static NSError *ScrapSckWindowsNotReadyError(NSUInteger found, size_t requested) {
    NSString *message = [NSString stringWithFormat:
        @"Screen frame windows are not ready (%lu of %zu found)",
        (unsigned long)found,
        requested];
    return [NSError errorWithDomain:kScrapSckErrorDomain
                               code:kScrapSckWindowsNotReadyErrorCode
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
    BOOL *filterReady,
    NSError **outputError) {
    if (filterReady != NULL) {
        *filterReady = NO;
    }
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
        if (filterReady != NULL) {
            *filterReady = YES;
        }
        return [[filterClass alloc] initWithDisplay:display excludingWindows:windows];
    }

    if (windowCount > 0 && !initial) {
        if (outputError != NULL) {
            *outputError = ScrapSckWindowsNotReadyError(windows.count, windowCount);
        }
        return nil;
    }

    if (initial) {
        NSArray<SCRunningApplication *> *applications =
            ScrapSckFindOwnApplications(content);
        if (windowCount == 0 && filterReady != NULL) {
            *filterReady = YES;
        }
        return [[filterClass alloc]
            initWithDisplay:display
            excludingApplications:applications
                 exceptingWindows:@[]];
    }

    if (filterReady != NULL) {
        *filterReady = YES;
    }
    return [[filterClass alloc] initWithDisplay:display excludingWindows:@[]];
}

@interface ScrapSckCapture : NSObject

@property(nonatomic, strong) SCStream *stream;
@property(nonatomic, strong) dispatch_queue_t queue;
@property(nonatomic, strong) dispatch_queue_t filterQueue;
@property(nonatomic, strong) NSError *filterUpdateError;
@property(nonatomic, assign) CGDirectDisplayID displayID;
@property(nonatomic, assign) ScrapSckFrameCallback frameCallback;
@property(nonatomic, assign) ScrapSckErrorCallback errorCallback;
@property(nonatomic, assign) void *callbackContext;
@property(nonatomic, assign) ScrapSckUpdateResult filterUpdateResult;
@property(nonatomic, assign) BOOL started;
@property(nonatomic, assign) BOOL shuttingDown;

- (BOOL)startWithDisplayID:(CGDirectDisplayID)displayID
                     width:(size_t)width
                    height:(size_t)height
                    cursor:(BOOL)cursor
         excludedWindowIDs:(const uint32_t *)windowIDs
               windowCount:(size_t)windowCount
               filterReady:(BOOL *)filterReady
                     error:(NSError **)outputError;
- (BOOL)applyExcludedWindowIDs:(const uint32_t *)windowIDs
                   windowCount:(size_t)windowCount
                         error:(NSError **)outputError;
- (ScrapSckUpdateResult)updateExcludedWindowIDs:(const uint32_t *)windowIDs
                                    windowCount:(size_t)windowCount
                                          error:(NSError **)outputError;
- (void)finishFilterUpdatesSynchronously;
- (BOOL)stopSynchronously:(NSError **)outputError;
- (void)invalidateCallbacksSynchronously;
- (BOOL)shutdownSynchronously:(NSError **)outputError;

@end

@implementation ScrapSckCapture

- (BOOL)startWithDisplayID:(CGDirectDisplayID)displayID
                     width:(size_t)width
                    height:(size_t)height
                    cursor:(BOOL)cursor
         excludedWindowIDs:(const uint32_t *)windowIDs
               windowCount:(size_t)windowCount
               filterReady:(BOOL *)filterReady
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
        content, displayID, windowIDs, windowCount, YES, filterReady, &error);
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
    dispatch_queue_set_specific(self.queue,
                                &kScrapSckCaptureQueueKey,
                                (__bridge void *)self,
                                NULL);
    self.filterQueue = dispatch_queue_create(
        "com.rustdesk.scrap.screencapturekit.filter", DISPATCH_QUEUE_SERIAL);
    dispatch_queue_set_specific(self.filterQueue,
                                &kScrapSckFilterQueueKey,
                                (__bridge void *)self,
                                NULL);
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
        self.filterQueue = nil;
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
        return NO;
    }
    if (startError != nil) {
        if (outputError != NULL) {
            *outputError = startError;
        }
        return NO;
    }
    return YES;
}

- (BOOL)applyExcludedWindowIDs:(const uint32_t *)windowIDs
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
        content, self.displayID, windowIDs, windowCount, NO, NULL, &error);
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

- (ScrapSckUpdateResult)updateExcludedWindowIDs:(const uint32_t *)windowIDs
                                    windowCount:(size_t)windowCount
                                          error:(NSError **)outputError {
    @synchronized (self) {
        ScrapSckUpdateResult result = self.filterUpdateResult;
        if (result == ScrapSckUpdatePending) {
            return result;
        }
        if (result != ScrapSckUpdateIdle) {
            if (outputError != NULL) {
                *outputError = self.filterUpdateError;
            }
            self.filterUpdateResult = ScrapSckUpdateIdle;
            self.filterUpdateError = nil;
            return result;
        }
        if (self.shuttingDown || self.filterQueue == nil) {
            if (outputError != NULL) {
                *outputError = ScrapSckErrorWithMessage(
                    @"Screen capture filter updates are unavailable");
            }
            return ScrapSckUpdateFailed;
        }

        NSData *windowIDData = windowCount == 0
            ? [NSData data]
            : [NSData dataWithBytes:windowIDs length:windowCount * sizeof(uint32_t)];
        self.filterUpdateResult = ScrapSckUpdatePending;
        dispatch_async(self.filterQueue, ^{
            @autoreleasepool {
                NSError *error = nil;
                const uint32_t *requestedWindowIDs = windowIDData.length == 0
                    ? NULL
                    : windowIDData.bytes;
                BOOL applied = [self applyExcludedWindowIDs:requestedWindowIDs
                                                windowCount:windowCount
                                                      error:&error];
                ScrapSckUpdateResult updateResult = ScrapSckUpdateApplied;
                if (!applied) {
                    if ([error.domain isEqualToString:kScrapSckErrorDomain] &&
                        error.code == kScrapSckWindowsNotReadyErrorCode) {
                        updateResult = ScrapSckUpdateNotReady;
                    } else {
                        updateResult = ScrapSckUpdateFailed;
                    }
                }
                @synchronized (self) {
                    self.filterUpdateError = error;
                    self.filterUpdateResult = updateResult;
                }
            }
        });
        return ScrapSckUpdatePending;
    }
}

- (void)finishFilterUpdatesSynchronously {
    dispatch_queue_t queue = nil;
    @synchronized (self) {
        self.shuttingDown = YES;
        queue = self.filterQueue;
    }
    if (queue != nil &&
        dispatch_get_specific(&kScrapSckFilterQueueKey) != (__bridge void *)self) {
        dispatch_sync(queue, ^{});
    }
}

- (BOOL)stopSynchronously:(NSError **)outputError {
    if (self.stream == nil) {
        return YES;
    }
    NSError *firstError = nil;
    BOOL stopped = !self.started;
    if (self.started) {
        dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
        __block NSError *stopError = nil;
        [self.stream stopCaptureWithCompletionHandler:^(NSError *error) {
            stopError = error;
            dispatch_semaphore_signal(semaphore);
        }];
        if (!ScrapSckWait(semaphore)) {
            firstError = ScrapSckErrorWithMessage(
                @"Timed out while stopping screen capture");
        } else if (stopError != nil) {
            firstError = stopError;
        } else {
            self.started = NO;
            stopped = YES;
        }
    }
    NSError *removeError = nil;
    BOOL removed = [self.stream removeStreamOutput:(id)self
                                               type:SCStreamOutputTypeScreen
                                              error:&removeError];
    if (!removed && firstError == nil) {
        firstError = removeError ?: ScrapSckErrorWithMessage(
            @"Failed to remove screen capture output");
    }
    if (outputError != NULL) {
        *outputError = firstError;
    }
    return stopped && removed;
}

- (void)invalidateCallbacksSynchronously {
    dispatch_queue_t queue = self.queue;
    void (^invalidate)(void) = ^{
        @synchronized (self) {
            self.frameCallback = NULL;
            self.errorCallback = NULL;
            self.callbackContext = NULL;
        }
    };
    if (queue != nil &&
        dispatch_get_specific(&kScrapSckCaptureQueueKey) != (__bridge void *)self) {
        dispatch_sync(queue, invalidate);
    } else {
        invalidate();
    }
}

- (BOOL)shutdownSynchronously:(NSError **)outputError {
    [self finishFilterUpdatesSynchronously];
    NSError *error = nil;
    BOOL stopped = [self stopSynchronously:&error];
    [self invalidateCallbacksSynchronously];
    if (stopped) {
        self.stream = nil;
        self.queue = nil;
        self.filterQueue = nil;
    }
    if (outputError != NULL) {
        *outputError = error;
    }
    return stopped;
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
    if (surface != NULL) {
        @synchronized (self) {
            ScrapSckFrameCallback callback = self.frameCallback;
            void *context = self.callbackContext;
            if (callback != NULL) {
                callback(context, surface);
            }
        }
    }
}

- (void)stream:(__unused SCStream *)stream didStopWithError:(NSError *)error {
    @synchronized (self) {
        ScrapSckErrorCallback callback = self.errorCallback;
        void *context = self.callbackContext;
        if (callback != NULL) {
            callback(context, error.localizedDescription.UTF8String);
        }
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
                       bool *outputFilterReady,
                       ScrapSckFrameCallback frameCallback,
                       ScrapSckErrorCallback errorCallback,
                       void *callbackContext,
                       ScrapSckError *outputError) {
    if (!scrap_sck_is_available()) {
        ScrapSckSetError(outputError, nil, @"ScreenCaptureKit is unavailable");
        return NULL;
    }
    @autoreleasepool {
        if (outputFilterReady != NULL) {
            *outputFilterReady = false;
        }
        ScrapSckCapture *capture = [[ScrapSckCapture alloc] init];
        capture.frameCallback = frameCallback;
        capture.errorCallback = errorCallback;
        capture.callbackContext = callbackContext;
        NSError *error = nil;
        BOOL filterReady = NO;
        if (![capture startWithDisplayID:displayID
                                   width:width
                                  height:height
                                  cursor:cursor
                       excludedWindowIDs:windowIDs
                             windowCount:windowCount
                             filterReady:&filterReady
                                   error:&error]) {
            NSError *shutdownError = nil;
            if (![capture shutdownSynchronously:&shutdownError]) {
                NSString *startMessage = error.localizedDescription ?: @"Failed to start screen capture";
                NSString *shutdownMessage = shutdownError.localizedDescription ?: @"unknown shutdown error";
                error = ScrapSckErrorWithMessage([NSString stringWithFormat:
                    @"%@; cleanup failed: %@", startMessage, shutdownMessage]);
                // Keep the delegate alive if ScreenCaptureKit could not be stopped safely.
                (void)CFBridgingRetain(capture);
            }
            ScrapSckSetError(outputError, error, @"Failed to start screen capture");
            return NULL;
        }
        if (outputFilterReady != NULL) {
            *outputFilterReady = filterReady == YES;
        }
        return (__bridge_retained void *)capture;
    }
}

int32_t scrap_sck_update_excluded_windows(void *handle,
                                          const uint32_t *windowIDs,
                                          size_t windowCount,
                                          ScrapSckError *outputError) {
    if (handle == NULL) {
        ScrapSckSetError(outputError, nil, @"Screen capture is unavailable");
        return ScrapSckUpdateFailed;
    }
    @autoreleasepool {
        ScrapSckCapture *capture = (__bridge ScrapSckCapture *)handle;
        NSError *error = nil;
        ScrapSckUpdateResult result =
            [capture updateExcludedWindowIDs:windowIDs
                                  windowCount:windowCount
                                        error:&error];
        if (result == ScrapSckUpdateFailed) {
            ScrapSckSetError(outputError, error, @"Failed to update screen capture filter");
        }
        return result;
    }
}

bool scrap_sck_destroy(void *handle, ScrapSckError *outputError) {
    if (handle == NULL) {
        return true;
    }
    @autoreleasepool {
        ScrapSckCapture *capture = (__bridge ScrapSckCapture *)handle;
        NSError *error = nil;
        if (![capture shutdownSynchronously:&error]) {
            ScrapSckSetError(outputError, error, @"Failed to stop screen capture safely");
            return false;
        }
        CFBridgingRelease(handle);
        return true;
    }
}
