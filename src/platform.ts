/** True when running in the macOS WebKit webview. */
export function isMacOS(): boolean {
  return /Mac/i.test(navigator.userAgent);
}
