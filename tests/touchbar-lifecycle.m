#import <AppKit/AppKit.h>
#include <assert.h>
void codex_touchbar_update(void *, bool, const char *);
// Objective-C polymorphism lets us exercise ownership without opening a native window.
@interface TestWindow : NSObject
@property(nonatomic, strong) NSTouchBar *touchBar;
@end
@implementation TestWindow
@end
int main(void) {
    @autoreleasepool {
        assert([NSThread isMainThread]);
        TestWindow *window = [TestWindow new];
        codex_touchbar_update((__bridge void *)window, false, "");
        assert(window.touchBar == nil);
        for (int i = 0; i < 20; i++) {
            codex_touchbar_update((__bridge void *)window, true, "Codex 75%");
            NSTouchBar *first = window.touchBar;
            assert(first != nil);
            codex_touchbar_update((__bridge void *)window, true, "Codex 50%");
            assert(first == window.touchBar);
            codex_touchbar_update((__bridge void *)window, false, "");
            assert(window.touchBar == nil);
        }
        puts("Touch Bar lifecycle: 20 enable/update/disable cycles passed (no hardware display test)");
    }
    return 0;
}
