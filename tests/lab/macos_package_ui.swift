// External OS accessibility client. Never linked into or bundled with the app.
import AppKit
import ApplicationServices
import Foundation

struct CheckFailure: Error, CustomStringConvertible {
  let description: String
}
func require(_ value: Bool, _ message: String) throws {
  if !value { throw CheckFailure(description: message) }
}
func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
  var value: CFTypeRef?
  guard AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success else { return nil }
  return value
}
func children(_ element: AXUIElement) -> [AXUIElement] {
  return attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? []
}
func find(_ root: AXUIElement, _ text: String) -> AXUIElement? {
  var queue = [root]
  var visited = Set<CFHashCode>()
  var count = 0
  while !queue.isEmpty && count < 10000 {
    let element = queue.removeLast()
    guard visited.insert(CFHash(element)).inserted else { continue }
    count += 1
    for key in [kAXTitleAttribute, kAXDescriptionAttribute, kAXValueAttribute] {
      if let value = attribute(element, key) as? String, value == text { return element }
    }
    queue.append(contentsOf: children(element).reversed())
  }
  return nil
}
func waitFor(_ root: AXUIElement, _ text: String) throws -> AXUIElement {
  let deadline = Date().addingTimeInterval(25)
  while Date() < deadline {
    if let item = find(root, text) { return item }
    RunLoop.current.run(until: Date().addingTimeInterval(0.2))
  }
  throw CheckFailure(description: "Native accessible element missing: \(text)")
}
func press(_ root: AXUIElement, _ text: String) throws {
  let item = try waitFor(root, text)
  try require(AXUIElementPerformAction(item, kAXPressAction as CFString) == .success, "Native press failed: \(text)")
}

let args = CommandLine.arguments
if args.count != 4 { fatalError("usage: package-ui app-path report-path finder|diagnostics") }
let appPath = URL(fileURLWithPath: args[1]).resolvingSymlinksInPath().path
let reportPath = URL(fileURLWithPath: args[2])
var report: [String: Any] = ["passed": false, "mode": args[3], "accessibilityTrusted": AXIsProcessTrusted()]
var application: NSRunningApplication?
do {
  try require(AXIsProcessTrusted(), "Native accessibility permission unavailable; do not bypass OS controls")
  let deadline = Date().addingTimeInterval(30)
  while Date() < deadline {
    application = NSRunningApplication.runningApplications(withBundleIdentifier: "dev.containerdesk.app").first {
      $0.bundleURL?.resolvingSymlinksInPath().path == appPath
    }
    if application != nil { break }
    RunLoop.current.run(until: Date().addingTimeInterval(0.2))
  }
  try require(application != nil, "Installed application did not start")
  let app = application!
  let root = AXUIElementCreateApplication(app.processIdentifier)
  // Standard accessibility activation for the owned WKWebView, under existing TCC grant.
  AXUIElementSetAttributeValue(root, "AXManualAccessibility" as CFString, kCFBooleanTrue)
  _ = try waitFor(root, "Settings")
  let windows = attribute(root, kAXWindowsAttribute) as? [AXUIElement] ?? []
  try require(!windows.isEmpty, "No native application window")
  report["nativeWindow"] = true
  report["installedBundlePath"] = app.bundleURL?.path
  if args[3] == "diagnostics" {
    try press(root, "Settings")
    try press(root, "Run diagnostics")
    _ = try waitFor(root, "OpenSSH: Available")
    _ = try waitFor(root, "/usr/bin/ssh")
    _ = try waitFor(root, "The inherited SSH agent socket is reachable. Loaded keys were not checked; a selected host may use a different IdentityAgent.")
    report["nativeOpenSshAvailable"] = true
    report["ownedAgentSocketReachable"] = true
  }
  let windowRows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
  if let row = windowRows.first(where: { ($0[kCGWindowOwnerPID as String] as? Int32) == app.processIdentifier && ($0[kCGWindowLayer as String] as? Int) == 0 }),
     let number = row[kCGWindowNumber as String] as? Int {
    let screenshot = Process()
    screenshot.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    screenshot.arguments = ["-x", "-l", String(number), reportPath.deletingPathExtension().appendingPathExtension("png").path]
    try screenshot.run(); screenshot.waitUntilExit()
    report["screenshotExit"] = screenshot.terminationStatus
  }
  try require(app.terminate(), "Normal application termination request failed")
  let closeDeadline = Date().addingTimeInterval(15)
  while !app.isTerminated && Date() < closeDeadline { RunLoop.current.run(until: Date().addingTimeInterval(0.1)) }
  try require(app.isTerminated, "Application did not terminate normally")
  report["normalTermination"] = true
  report["passed"] = true
} catch {
  report["error"] = String(describing: error)
  // Bound failure cleanup to this exact installed application only.
  if let app = application, !app.isTerminated { app.terminate() }
}
let data = try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
try data.write(to: reportPath)
print(String(decoding: data, as: UTF8.self))
exit((report["passed"] as? Bool) == true ? 0 : 1)
