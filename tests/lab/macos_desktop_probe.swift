import AppKit
import ApplicationServices
import Foundation

let report: [String: Any] = [
  "accessibilityTrusted": AXIsProcessTrusted(),
  "screenCaptureAllowed": CGPreflightScreenCaptureAccess(),
  "windowServerAvailable": CGSessionCopyCurrentDictionary() != nil,
  "operatingSystem": ProcessInfo.processInfo.operatingSystemVersionString,
  "finderRunning": NSRunningApplication.runningApplications(withBundleIdentifier: "com.apple.finder").count > 0,
]
let data = try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
print(String(decoding: data, as: UTF8.self))
