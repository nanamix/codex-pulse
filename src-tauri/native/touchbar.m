#import <AppKit/AppKit.h>
#import <objc/runtime.h>

static char UsageTouchBarKey;
static NSTouchBarItemIdentifier const UsageItem = @"com.nanamix.codex-usage-monitor.usage";
@interface UsageTouchBarController : NSObject <NSTouchBarDelegate>
@property(nonatomic, strong) NSTouchBar *bar;
@property(nonatomic, strong) NSTextField *label;
@property(nonatomic, copy) NSString *text;
@end
@implementation UsageTouchBarController
- (instancetype)init {
    if ((self = [super init])) {
        _text = @"Codex 정보 없음";
        _bar = [[NSTouchBar alloc] init];
        _bar.delegate = self;
        _bar.defaultItemIdentifiers = @[UsageItem];
    }
    return self;
}
- (NSTouchBarItem *)touchBar:(NSTouchBar *)touchBar makeItemForIdentifier:(NSTouchBarItemIdentifier)identifier {
    if (![identifier isEqualToString:UsageItem]) return nil;
    NSCustomTouchBarItem *item = [[NSCustomTouchBarItem alloc] initWithIdentifier:identifier];
    self.label = [NSTextField labelWithString:self.text];
    item.view = self.label;
    return item;
}
@end
void codex_touchbar_update(void *pointer, bool enabled, const char *text) {
    @autoreleasepool {
        NSWindow *window = (__bridge NSWindow *)pointer;
        if (!window || ![NSThread isMainThread]) return;
        if (!enabled) {
            window.touchBar = nil;
            objc_setAssociatedObject(window, &UsageTouchBarKey, nil, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
            return;
        }
        UsageTouchBarController *controller = objc_getAssociatedObject(window, &UsageTouchBarKey);
        if (!controller) {
            controller = [[UsageTouchBarController alloc] init];
            objc_setAssociatedObject(window, &UsageTouchBarKey, controller, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
            window.touchBar = controller.bar;
        }
        controller.text = text ? ([NSString stringWithUTF8String:text] ?: @"Codex 정보 없음") : @"Codex 정보 없음";
        if (controller.label) controller.label.stringValue = controller.text;
    }
}
