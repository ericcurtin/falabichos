// SPDX-License-Identifier: Apache-2.0
// Entry point: hands control to the Rust game (`main_rs` in crates/falabichos/src/lib.rs).
// winit's iOS backend starts UIApplicationMain itself and never returns.
#import <UIKit/UIKit.h>

extern void main_rs(void);

int main(int argc, char *argv[]) {
    @autoreleasepool {
        main_rs();
    }
    return 0;
}
