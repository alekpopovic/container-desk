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
if args.count != 4 { fatalError("usage: package-ui app-path report-path finder|diagnostics|support") }
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
    _ = try waitFor(root, "OpenSSH is available.")
    _ = try waitFor(root, "/usr/bin/ssh")
    _ = try waitFor(root, "The inherited SSH agent socket is reachable. Loaded keys were not checked; a selected host may use a different IdentityAgent.")
    report["nativeOpenSshAvailable"] = true
    report["ownedAgentSocketReachable"] = true
  }
  if args[3] == "support" {
    try press(root, "Settings")
    try press(root, "Prepare support preview")
    let destination = reportPath.deletingLastPathComponent().appendingPathComponent("containerdesk-support.json")
    try require(!FileManager.default.fileExists(atPath: destination.path), "Support destination must be new")
    try press(root, "Save reviewed report…")
    _ = try waitFor(root, "Save")
    try require(app.activate(options: [.activateIgnoringOtherApps]), "Cannot activate owned save dialog")
    func key(_ code: CGKeyCode, _ flags: CGEventFlags = []) throws {
      guard let down = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true),
            let up = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false) else {
        throw CheckFailure(description: "Cannot create native keyboard event")
      }
      down.flags = flags; up.flags = flags
      down.postToPid(app.processIdentifier); up.postToPid(app.processIdentifier)
      RunLoop.current.run(until: Date().addingTimeInterval(0.3))
    }
    // NSSavePanel's native Go to Folder sheet. Events target only the owned app PID.
    try key(5, [.maskCommand, .maskShift])
    RunLoop.current.run(until: Date().addingTimeInterval(0.5))
    guard let focused = attribute(root, kAXFocusedUIElementAttribute),
          CFGetTypeID(focused) == AXUIElementGetTypeID() else {
      throw CheckFailure(description: "Native save folder field missing")
    }
    try require(AXUIElementSetAttributeValue(focused as! AXUIElement, kAXValueAttribute as CFString,
      reportPath.deletingLastPathComponent().path as CFString) == .success, "Cannot select owned export folder")
    try key(36)
    RunLoop.current.run(until: Date().addingTimeInterval(0.5))
    try press(root, "Save")
    let saveDeadline = Date().addingTimeInterval(15)
    while !FileManager.default.fileExists(atPath: destination.path) && Date() < saveDeadline {
      RunLoop.current.run(until: Date().addingTimeInterval(0.1))
    }
    let exported = try Data(contentsOf: destination)
    let payload = try JSONSerialization.jsonObject(with: exported) as? [String: Any]
    try require(payload?["schemaVersion"] as? Int == 1, "Native saved report is invalid")
    _ = try waitFor(root, "Reviewed support report saved.")
    report["nativeSupportSaveDialog"] = true
    report["savedSupportBytes"] = exported.count
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
  if let app = application {
    var queue = [AXUIElementCreateApplication(app.processIdentifier)]
    var visited = Set<CFHashCode>()
    var labels: [[String: String]] = []
    while !queue.isEmpty && visited.count < 2000 {
      let element = queue.removeLast()
      guard visited.insert(CFHash(element)).inserted else { continue }
      var entry: [String: String] = [:]
      for key in [kAXRoleAttribute, kAXTitleAttribute, kAXDescriptionAttribute, kAXValueAttribute] {
        if let value = attribute(element, key) as? String, !value.isEmpty {
          entry[key] = String(value.prefix(1000))
        }
      }
      if !entry.isEmpty { labels.append(entry) }
      queue.append(contentsOf: children(element).reversed())
    }
    report["ownedFreshProfileAccessibility"] = labels
  }
  // Bound failure cleanup to this exact installed application only.
  if let app = application, !app.isTerminated { app.terminate() }
}
let data = try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
try data.write(to: reportPath)
print(String(decoding: data, as: UTF8.self))
exit((report["passed"] as? Bool) == true ? 0 : 1)
