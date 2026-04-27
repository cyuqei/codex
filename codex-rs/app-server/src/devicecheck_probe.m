#import <Foundation/Foundation.h>
#import <dispatch/dispatch.h>
#import <dlfcn.h>
#import <objc/message.h>
#import <objc/runtime.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

struct CodexDeviceCheckProbeResult {
    bool supported;
    bool has_token;
    unsigned long token_length;
    char *token_base64;
    char *error_description;
};

static char *codex_strdup_nsstring(NSString *string) {
    if (string == nil) {
        return NULL;
    }
    const char *utf8 = [string UTF8String];
    if (utf8 == NULL) {
        return NULL;
    }
    return strdup(utf8);
}

struct CodexDeviceCheckProbeResult codex_devicecheck_probe(void) {
    @autoreleasepool {
        struct CodexDeviceCheckProbeResult result = {
            .supported = false,
            .has_token = false,
            .token_length = 0,
            .token_base64 = NULL,
            .error_description = NULL,
        };

        Class deviceClass = NSClassFromString(@"DCDevice");
        if (deviceClass == nil) {
            dlopen("/System/Library/Frameworks/DeviceCheck.framework/DeviceCheck", RTLD_LAZY | RTLD_LOCAL);
            deviceClass = NSClassFromString(@"DCDevice");
        }
        if (deviceClass == nil) {
            result.error_description = strdup("DeviceCheck.framework is not available");
            return result;
        }

        id (*currentDevice)(Class, SEL) = (id (*)(Class, SEL))objc_msgSend;
        id device = currentDevice(deviceClass, sel_registerName("currentDevice"));
        if (device == nil) {
            result.error_description = strdup("DCDevice currentDevice returned nil");
            return result;
        }

        BOOL (*isSupported)(id, SEL) = (BOOL (*)(id, SEL))objc_msgSend;
        result.supported = isSupported(device, sel_registerName("isSupported"));
        if (!result.supported) {
            return result;
        }

        __block NSData *token = nil;
        __block NSError *deviceCheckError = nil;
        dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
        void (^completionHandler)(NSData * _Nullable, NSError * _Nullable) = ^(
            NSData * _Nullable data, NSError * _Nullable error
        ) {
            token = [data retain];
            deviceCheckError = [error retain];
            dispatch_semaphore_signal(semaphore);
        };
        void (*generateToken)(id, SEL, void (^)(NSData * _Nullable, NSError * _Nullable)) =
            (void (*)(id, SEL, void (^)(NSData * _Nullable, NSError * _Nullable)))objc_msgSend;
        generateToken(device, sel_registerName("generateTokenWithCompletionHandler:"), completionHandler);

        dispatch_time_t timeout = dispatch_time(DISPATCH_TIME_NOW, 30 * NSEC_PER_SEC);
        if (dispatch_semaphore_wait(semaphore, timeout) != 0) {
            result.error_description = strdup("timed out waiting for DeviceCheck token");
            return result;
        }

        if (token != nil) {
            result.has_token = true;
            result.token_length = [token length];
            NSString *base64 = [token base64EncodedStringWithOptions:0];
            result.token_base64 = codex_strdup_nsstring(base64);
        }
        if (deviceCheckError != nil) {
            result.error_description = codex_strdup_nsstring([deviceCheckError description]);
        }

        [token release];
        [deviceCheckError release];
        return result;
    }
}

void codex_devicecheck_probe_free(char *string) {
    free(string);
}
