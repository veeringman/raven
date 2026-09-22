import RavenKit
import SwiftUI

private enum Brand {
    static let night = Color(red: 0.027, green: 0.031, blue: 0.047) // #07080C
    static let plumage = Color(red: 0.090, green: 0.106, blue: 0.157) // #171B28
    static let mist = Color(red: 0.957, green: 0.965, blue: 0.984) // #F4F6FB
    static let silver = Color(red: 0.604, green: 0.639, blue: 0.722) // #9AA3B8
    static let amber = Color(red: 0.961, green: 0.725, blue: 0.259) // #F5B942
}

struct ContentView: View {
    @State private var snapshot: FfiRunSnapshot?
    @State private var running = false

    var body: some View {
        ZStack {
            background
            ScrollView {
                VStack(alignment: .leading, spacing: 28) {
                    header
                    if let snapshot {
                        outcome(snapshot)
                        traceList(snapshot)
                        actions(snapshot)
                    } else {
                        idle
                    }
                }
                .padding(.horizontal, 24)
                .padding(.top, 56)
                .padding(.bottom, 40)
            }
        }
        .preferredColorScheme(.dark)
    }

    private var background: some View {
        LinearGradient(
            colors: [Brand.night, Brand.plumage, Brand.night],
            startPoint: .topLeading,
            endPoint: .bottomTrailing
        )
        .ignoresSafeArea()
        .overlay {
            Circle()
                .fill(Brand.amber.opacity(0.08))
                .frame(width: 280, height: 280)
                .blur(radius: 60)
                .offset(x: 120, y: -180)
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("RAVEN")
                .font(.system(size: 34, weight: .semibold, design: .rounded))
                .tracking(6)
                .foregroundStyle(Brand.mist)
            Text("From intent to action")
                .font(.system(size: 13, weight: .medium, design: .rounded))
                .tracking(2)
                .foregroundStyle(Brand.silver)
            Text("Prepare my day")
                .font(.system(size: 28, weight: .semibold, design: .rounded))
                .foregroundStyle(Brand.mist)
                .padding(.top, 12)
            Text("One intent. On-device Foundation Models proposes a plan; RAVEN authorizes, acts, verifies — and asks before sending.")
                .font(.system(size: 16, weight: .regular, design: .rounded))
                .foregroundStyle(Brand.silver)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private var idle: some View {
        Button(action: start) {
            Text(running ? "Running…" : "Run intent")
                .font(.system(size: 17, weight: .semibold, design: .rounded))
                .foregroundStyle(Brand.night)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 16)
                .background(Brand.amber)
        }
        .disabled(running)
        .padding(.top, 8)
    }

    private func outcome(_ snapshot: FfiRunSnapshot) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Outcome")
                .font(.system(size: 12, weight: .semibold, design: .rounded))
                .tracking(1.5)
                .foregroundStyle(Brand.silver)
            Text(stateLabel(snapshot.state))
                .font(.system(size: 22, weight: .semibold, design: .rounded))
                .foregroundStyle(Brand.mist)
            if let pending = snapshot.pendingCapabilityId {
                Text("Waiting on \(pending)")
                    .font(.system(size: 15, weight: .medium, design: .rounded))
                    .foregroundStyle(Brand.amber)
            }
        }
    }

    private func traceList(_ snapshot: FfiRunSnapshot) -> some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Trace")
                .font(.system(size: 12, weight: .semibold, design: .rounded))
                .tracking(1.5)
                .foregroundStyle(Brand.silver)
            ForEach(Array(snapshot.trace.enumerated()), id: \.offset) { _, entry in
                VStack(alignment: .leading, spacing: 4) {
                    Text(entry.phase.uppercased())
                        .font(.system(size: 11, weight: .bold, design: .rounded))
                        .tracking(1.2)
                        .foregroundStyle(Brand.amber.opacity(0.9))
                    Text(entry.summary)
                        .font(.system(size: 15, weight: .regular, design: .rounded))
                        .foregroundStyle(Brand.mist)
                    if let policy = entry.policy {
                        Text("policy \(policy)")
                            .font(.system(size: 13, weight: .regular, design: .rounded))
                            .foregroundStyle(Brand.silver)
                    }
                    if let verified = entry.verified {
                        Text(verified ? "verified" : "not verified")
                            .font(.system(size: 13, weight: .medium, design: .rounded))
                            .foregroundStyle(verified ? Brand.amber : Brand.silver)
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func actions(_ snapshot: FfiRunSnapshot) -> some View {
        if snapshot.state == .waitingForUser {
            VStack(spacing: 12) {
                Button(action: { resume(.approve) }) {
                    actionLabel("Approve send")
                        .foregroundStyle(Brand.night)
                        .background(Brand.amber)
                }
                Button(action: { resume(.deny) }) {
                    actionLabel("Deny")
                        .foregroundStyle(Brand.mist)
                        .background(Brand.plumage)
                }
                Button(action: { resume(.cancel) }) {
                    actionLabel("Cancel run")
                        .foregroundStyle(Brand.silver)
                        .background(Color.clear)
                        .overlay {
                            RoundedRectangle(cornerRadius: 0)
                                .stroke(Brand.silver.opacity(0.35), lineWidth: 1)
                        }
                }
            }
            .padding(.top, 8)
        } else {
            Button(action: start) {
                actionLabel(running ? "Running…" : "Run again")
                    .foregroundStyle(Brand.night)
                    .background(Brand.amber)
            }
            .disabled(running)
            .padding(.top, 8)
        }
    }

    private func actionLabel(_ title: String) -> some View {
        Text(title)
            .font(.system(size: 17, weight: .semibold, design: .rounded))
            .frame(maxWidth: .infinity)
            .padding(.vertical, 16)
    }

    private func start() {
        running = true
        Task {
            let result = await Raven.startPrepareMyDay(planner: FoundationModelsDayPlanner())
            snapshot = result
            running = false
        }
    }

    private func resume(_ reply: FfiUserReply) {
        running = true
        Task {
            let result = await Task.detached {
                Raven.resume(reply: reply)
            }.value
            snapshot = result
            running = false
        }
    }

    private func stateLabel(_ state: FfiGoalState) -> String {
        switch state {
        case .waitingForUser: return "Waiting for you"
        case .completed: return "Completed"
        case .failed: return "Failed"
        case .cancelled: return "Cancelled"
        default: return String(describing: state)
        }
    }
}

#Preview {
    ContentView()
}
