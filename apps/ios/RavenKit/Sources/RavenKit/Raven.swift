import Foundation
import FoundationModels

/// Result of a host-side plan proposal.
public struct DayPlanProposal: Sendable {
    public let provider: String
    public let steps: [FfiProposedStep]

    public init(provider: String, steps: [FfiProposedStep]) {
        self.provider = provider
        self.steps = steps
    }
}

/// Host-side planner. The Rust runtime still authorizes and verifies; this
/// only proposes steps. Swap implementations without changing the loop.
public protocol DayPlanner: Sendable {
    func propose() async -> DayPlanProposal
}

/// Capability catalog the on-device model may choose from.
public enum DayCapabilityCatalog {
    public static let ids = [
        "read_calendar",
        "read_tasks",
        "read_weather",
        "read_location",
        "send_message",
    ]

    public static let claims: [String: String] = [
        "read_calendar": "calendar.loaded",
        "read_tasks": "tasks.loaded",
        "read_weather": "weather.loaded",
        "read_location": "location.loaded",
        "send_message": "message.sent",
    ]

    public static let summaries: [String: String] = [
        "read_calendar": "Inspect the calendar",
        "read_tasks": "Inspect open tasks",
        "read_weather": "Check the weather",
        "read_location": "Check where you are",
        "send_message": "Send the day summary",
    ]

    public static func step(
        capabilityId: String,
        summary: String? = nil
    ) -> FfiProposedStep? {
        guard let claim = claims[capabilityId] else { return nil }
        return FfiProposedStep(
            capabilityId: capabilityId,
            summary: summary?.isEmpty == false ? summary! : (summaries[capabilityId] ?? capabilityId),
            expectedClaim: claim,
            input: ""
        )
    }
}

/// Deterministic plan matching the in-process demo.
public struct ScriptedDayPlanner: DayPlanner {
    public init() {}

    public func propose() async -> DayPlanProposal {
        DayPlanProposal(provider: "scripted", steps: Self.steps)
    }

    public static var steps: [FfiProposedStep] {
        DayCapabilityCatalog.ids.compactMap { DayCapabilityCatalog.step(capabilityId: $0) }
    }
}

@available(iOS 26.0, *)
@Generable
struct FoundationDayPlan {
    @Guide(description: "Ordered steps for preparing the person's day", .count(5))
    var steps: [FoundationDayStep]
}

@available(iOS 26.0, *)
@Generable
struct FoundationDayStep {
    @Guide(
        description: "Capability id from the RAVEN registry",
        .anyOf([
            "read_calendar",
            "read_tasks",
            "read_weather",
            "read_location",
            "send_message",
        ])
    )
    var capabilityId: String

    @Guide(description: "Short human summary of the step")
    var summary: String
}

/// On-device Apple Foundation Models planner. Falls back to the scripted plan
/// when the system model is unavailable or generation fails.
public struct FoundationModelsDayPlanner: DayPlanner {
    private let intent: String

    public init(intent: String = "Prepare my day.") {
        self.intent = intent
    }

    public func propose() async -> DayPlanProposal {
        await proposeWithFoundationModels()
    }

    @available(iOS 26.0, *)
    private func generateWithSystemModel() async throws -> [FfiProposedStep] {
        let model = SystemLanguageModel.default
        guard model.isAvailable else {
            throw PlannerError.modelUnavailable
        }

        let session = LanguageModelSession(
            model: model,
            instructions: """
            You are the planner for RAVEN, an agentic application runtime.
            Propose an ordered plan using only capability ids from the registry.
            Prefer observation first: calendar, tasks, weather, location.
            A message step is external communication and will require user approval.
            Never invent capability ids. Never call tools yourself.
            """
        )

        let response = try await session.respond(
            to: """
            Intent: \(intent)
            Available capabilities: \(DayCapabilityCatalog.ids.joined(separator: ", "))
            Propose the best ordered plan for this intent.
            """,
            generating: FoundationDayPlan.self
        )

        let mapped = response.content.steps.compactMap { step in
            DayCapabilityCatalog.step(capabilityId: step.capabilityId, summary: step.summary)
        }
        if mapped.isEmpty {
            throw PlannerError.emptyPlan
        }
        return mapped
    }

    private func proposeWithFoundationModels() async -> DayPlanProposal {
        if #available(iOS 26.0, *) {
            do {
                let steps = try await generateWithSystemModel()
                return DayPlanProposal(provider: "foundation-models", steps: steps)
            } catch {
                return DayPlanProposal(
                    provider: "scripted-fallback",
                    steps: ScriptedDayPlanner.steps
                )
            }
        } else {
            return DayPlanProposal(
                provider: "scripted-fallback",
                steps: ScriptedDayPlanner.steps
            )
        }
    }
}

private enum PlannerError: Error {
    case modelUnavailable
    case emptyPlan
}

/// Thin façade over the UniFFI bridge for the sample app.
public enum Raven {
    public static func startPrepareMyDay(
        planner: any DayPlanner = FoundationModelsDayPlanner()
    ) async -> FfiRunSnapshot {
        let proposal = await planner.propose()
        return startPrepareMyDayWithSteps(provider: proposal.provider, steps: proposal.steps)
    }

    public static func resume(reply: FfiUserReply) -> FfiRunSnapshot {
        resumePrepareMyDay(reply: reply)
    }

    public static func principles() -> [String] {
        ravenPrinciples()
    }
}
