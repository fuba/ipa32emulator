// SPDX-License-Identifier: MIT
// Copyright (c) 2026 IPA32 Emulator contributors

import SwiftUI

struct DungeonRaidCheckpointView: View {
    @Environment(\.dismiss) private var dismiss
    @State private var hasRun = false
    @State private var savedDates: [Int: Date] = [:]
    @State private var previousDate: Date?
    @State private var confirmation: Confirmation?
    @State private var errorMessage: String?
    let store: DungeonRaidCheckpoints

    init(documents: URL) {
        store = DungeonRaidCheckpoints(documents: documents)
    }

    private enum Confirmation {
        case overwrite(Int)
        case restore(Int)
        case restorePrevious

        var title: String {
            switch self {
            case .overwrite(let slot): "Overwrite Slot \(slot)?"
            case .restore(let slot): "Load Slot \(slot)?"
            case .restorePrevious: "Restore Previous Run?"
            }
        }
    }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    ForEach(DungeonRaidCheckpoints.slots, id: \.self) { slot in
                        HStack {
                            VStack(alignment: .leading) {
                                Text("Slot \(slot)")
                                if let date = savedDates[slot] {
                                    Text(date, format: .dateTime.month().day().hour().minute())
                                        .font(.subheadline)
                                        .foregroundStyle(.secondary)
                                } else {
                                    Text("Empty")
                                        .font(.subheadline)
                                        .foregroundStyle(.secondary)
                                }
                            }
                            Spacer()
                            Button("Save") {
                                if savedDates[slot] == nil {
                                    perform { try store.save(to: slot) }
                                } else {
                                    confirmation = .overwrite(slot)
                                }
                            }
                            .accessibilityIdentifier("save-checkpoint-\(slot)")
                            .disabled(!hasRun)
                            Button("Load") { confirmation = .restore(slot) }
                                .accessibilityIdentifier("load-checkpoint-\(slot)")
                                .disabled(savedDates[slot] == nil)
                        }
                        .buttonStyle(.bordered)
                    }
                } footer: {
                    Text("The game saves after each completed turn. Open Dungeon Raid and tap Continue after loading a slot.")
                }

                if previousDate != nil {
                    Section {
                        Button("Restore Previous Run") { confirmation = .restorePrevious }
                    }
                }
            }
            .navigationTitle("Dungeon Raid Checkpoints")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
        .onAppear(perform: refresh)
        .alert(confirmation?.title ?? "", isPresented: Binding(
            get: { confirmation != nil },
            set: { if !$0 { confirmation = nil } }
        ), presenting: confirmation) { action in
            switch action {
            case .overwrite(let slot):
                Button("Overwrite", role: .destructive) { perform { try store.save(to: slot) } }
            case .restore(let slot):
                Button("Load Checkpoint") {
                    perform(dismissOnSuccess: true) { try store.restore(from: slot) }
                }
            case .restorePrevious:
                Button("Restore") {
                    perform(dismissOnSuccess: true) { try store.restorePrevious() }
                }
            }
            Button("Cancel", role: .cancel) {}
        } message: { action in
            if case .restore = action {
                Text("Your current run will be kept under Restore Previous Run. Open Dungeon Raid and tap Continue to play the loaded run.")
            }
        }
        .alert("Checkpoint Error", isPresented: Binding(
            get: { errorMessage != nil },
            set: { if !$0 { errorMessage = nil } }
        )) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(errorMessage ?? "")
        }
    }

    private func refresh() {
        hasRun = (try? store.currentRun()) != nil
        savedDates = Dictionary(uniqueKeysWithValues: DungeonRaidCheckpoints.slots.compactMap { slot in
            (try? store.date(in: .number(slot))).map { (slot, $0) }
        })
        previousDate = try? store.date(in: .previous)
    }

    private func perform(dismissOnSuccess: Bool = false, _ operation: () throws -> Void) {
        do {
            try operation()
            if dismissOnSuccess {
                dismiss()
            } else {
                refresh()
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}
