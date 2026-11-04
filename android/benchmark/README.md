# benchmark

Macrobenchmarks of the demo app on a release build (R8 on — the only place smoothness is judged):

| Test | Measures |
| --- | --- |
| `startupNoProfile` | cold start, nothing precompiled |
| `startupBaselineProfile` | cold start with the Baseline Profile (should be faster) |
| `scroll` | frame times while scrolling the components |

```sh
gradle -p android :benchmark:connectedBenchmarkReleaseAndroidTest   # connected device or emulator, API 28+
```

Apps copy this module for their own journeys (start, open the main list, scroll it). CI only builds it.
