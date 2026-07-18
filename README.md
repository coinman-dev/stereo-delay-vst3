# Stereo Delay

VST3 effect with independent timing offsets for the left and right channels.

## Ready-to-use packages

- Windows: `plugins/windows/StereoDelay.vst3`
- Linux: `plugins/linux/StereoDelay.vst3`

Copy the complete `StereoDelay.vst3` directory, not only the file inside it.

On Windows, install it in `C:\\Program Files\\Common Files\\VST3\\` and rescan plugins in
PreSonus Studio One Pro. On Linux, install it in `~/.vst3/` and rescan in the VST3-compatible host.

## Parameters

- `Left Offset`: `-50.0` to `+50.0 ms`, default `0.0 ms`
- `Right Offset`: `-50.0` to `+50.0 ms`, default `0.0 ms`
- `Left Phase`: `-180` to `+180 deg` in `1 deg` steps, default `0 deg`
- `Right Phase`: `-180` to `+180 deg` in `1 deg` steps, default `0 deg`

The phase controls use a wideband Hilbert phase rotator. At `0 deg` the signal is unchanged;
`-180 deg` and `+180 deg` invert the selected channel. This is a broadband approximation, so the
result is least exact close to DC and Nyquist. When either phase control is non-zero, the plugin
reports an additional fixed 64-sample latency to keep both channels aligned.

Host latency compensation changes automatically to the minimum required value. For example, with
`Left Offset = -5 ms` and `Right Offset = +5 ms`, the plugin reports `5 ms` latency to the host,
then applies physical delays of `0 ms` to the left channel and `10 ms` to the right. When both
offsets are non-negative, the reported latency is `0 ms`.

The plugin intentionally has no custom editor. Studio One displays `Left Offset` and `Right
Offset` using its native controls, as shown in the insert panel, and exposes them to automation.

## Build

```bash
scripts/build-vst3.sh x86_64-pc-windows-gnu
scripts/build-vst3.sh x86_64-unknown-linux-gnu
```

The script writes the packages to `plugins/windows` and `plugins/linux`.

### Clean generated files

Remove local build artifacts, generated plugin bundles, dependency caches, temporary files, and the generated lockfile:

```bash
./scripts/clean.sh
```

Preview what will be removed without changing files:

```bash
./scripts/clean.sh --dry-run
```
