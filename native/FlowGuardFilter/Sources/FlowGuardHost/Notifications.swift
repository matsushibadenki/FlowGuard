import Foundation
import UserNotifications
private var revision: UInt64 = 0
private var epoch: UInt64 = 0
private let notificationID = "flowguard.pending-flows"
/// Optional ABI4 capability. Permission is requested only by the explicit settings action.
@_cdecl("flowguard_host_notification")
public func notification(_ operation: Int32, _ version: UInt64, _ bytes: UnsafePointer<UInt8>?, _ length: UInt64, _ callback: @escaping LifecycleCallback, _ token: UInt64) {
    guard canActivate() == 1 else { "unavailable".withCString { callback($0,token) }; return }
    guard length <= 1024 else { "unavailable".withCString { callback($0,token) }; return }
    let body = bytes.map { Data(bytes:$0,count:Int(length)) } ?? Data()
    DispatchQueue.main.async {
        let center = UNUserNotificationCenter.current()
        let reply: (String) -> Void = { status in status.withCString { callback($0,token) } }
        if operation == 3 {
            guard version >= revision else { reply("stale"); return }
            revision = version; epoch += 1
            center.removePendingNotificationRequests(withIdentifiers:[notificationID]); center.removeDeliveredNotifications(withIdentifiers:[notificationID]); reply("cleared"); return
        }
        if operation == 1 {
            center.requestAuthorization(options:[.alert]) { allowed, error in reply(error != nil ? "unknown" : allowed ? "authorized" : "denied") }; return
        }
        let captured = epoch
        center.getNotificationSettings { settings in
            DispatchQueue.main.async {
                let authorized = settings.authorizationStatus == .authorized || settings.authorizationStatus == .provisional
                guard operation == 2 else { reply(authorized ? "authorized" : settings.authorizationStatus == .notDetermined ? "not_requested" : "denied"); return }
                guard version >= revision, captured == epoch else { reply("stale"); return }
                guard authorized else { reply("denied"); return }
                guard let object = try? JSONSerialization.jsonObject(with:body) as? [String:Any], let count = object["count"] as? Int, count >= 0, count <= 128, let locale = object["locale"] as? String else { reply("unavailable"); return }
                let unknown = object["unknownCount"] as? Int ?? 0
                guard unknown >= 0, unknown <= 1024, count + unknown > 0 else { reply("unavailable"); return }
                revision = version
                let content = UNMutableNotificationContent(); content.title = "FlowGuard"
                content.body = locale == "ja" ? "通信の確認待ち：\(count)件。メニューバーからFlowGuardを開いてください。" : locale == "zh-CN" ? "待确认连接：\(count)。请从菜单栏打开FlowGuard。" : "\(count) flow decisions pending. Open FlowGuard from the menu bar."
                if unknown > 0 {
                    let observed = locale == "ja" ? "新しく観測したルール未登録のTCP接続：\(unknown)件。攻撃・許可・遮断の判定ではありません。" : locale == "zh-CN" ? "新观测到的未注册TCP连接：\(unknown)。不代表攻击、允许或阻止判定。" : "\(unknown) newly observed TCP endpoints without rules. Not an attack or traffic verdict."
                    content.body = observed + (count > 0 ? " " + content.body : "")
                }
                center.add(UNNotificationRequest(identifier:notificationID,content:content,trigger:nil)) { error in
                    DispatchQueue.main.async {
                        if captured != epoch { center.removePendingNotificationRequests(withIdentifiers:[notificationID]); center.removeDeliveredNotifications(withIdentifiers:[notificationID]) }
                        reply(error == nil ? "submitted" : "failed")
                    }
                }
            }
        }
    }
}
