package dev.krifmaz.kit.baselineprofile

import androidx.benchmark.macro.MacrobenchmarkScope
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Direction
import androidx.test.uiautomator.Until

/** The demo app's package. */
const val DEMO_PACKAGE = "dev.krifmaz.kit.demo"

/**
 * Scrolls the demo's list (resource id "demo-list") down and back up: the journey the profile covers and the scroll
 * benchmark measures.
 */
fun MacrobenchmarkScope.scrollDemo() {
    device.wait(Until.hasObject(By.res("demo-list")), 5_000)
    val list = device.findObject(By.res("demo-list")) ?: return
    // keep the fling away from the system's edge gestures
    list.setGestureMargin(device.displayWidth / 5)
    list.fling(Direction.DOWN)
    device.waitForIdle()
    list.fling(Direction.UP)
    device.waitForIdle()
}
