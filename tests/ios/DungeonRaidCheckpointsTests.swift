// SPDX-License-Identifier: MIT
// Copyright (c) 2026 IPA32 Emulator contributors
import Foundation

@main
struct CheckpointTests {
    static func main() throws {
        try withStore { store in
            let firstRun = Data([1, 2, 3])
            let secondRun = Data([4, 5, 6])
            try writePreferences(["save": firstRun, "name": "First"], to: store.preferences)
            try store.save(to: 1)
            try writePreferences(["save": secondRun, "name": "Second"], to: store.preferences)
            try store.save(to: 2)

            try store.restore(from: 1)
            try expect(store.currentRun() == firstRun, "Slot 1 replaces the current run")
            try expect(store.run(in: .previous) == secondRun, "The replaced run is preserved")
            try expect(store.run(in: .number(1)) == firstRun, "Loading keeps Slot 1 intact")
            try expect(store.run(in: .number(2)) == secondRun, "Slot 2 is independent")
            try expect(readPreferences(at: store.preferences)["name"] as? String == "Second",
                       "Loading keeps unrelated settings")

            try store.restorePrevious()
            try store.restorePrevious()
            try expect(store.currentRun() == secondRun, "The previous run can be loaded repeatedly")
            try expect(store.run(in: .previous) == secondRun, "The backup remains available")
        }
        try withStore { store in
            try expectFailure(.noCurrentRun) { try store.save(to: 1) }
            let run = Data([7, 8, 9])
            try writePreferences(["save": run], to: store.preferences)
            try expectFailure(.invalidCheckpoint) { try store.restore(from: 1) }
            try store.save(to: 1)
            try Data().write(to: store.url(for: .number(1)), options: .atomic)
            try expectFailure(.invalidCheckpoint) { try store.restore(from: 1) }
            try expect(store.currentRun() == run, "An invalid slot does not change the current run")
            try expect(store.run(in: .previous) == nil, "An invalid slot does not touch the backup")
        }
        print("Dungeon Raid checkpoint tests passed")
    }

    private static func withStore(_ test: (DungeonRaidCheckpoints) throws -> Void) rethrows {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: folder) }
        try test(DungeonRaidCheckpoints(documents: folder.appendingPathComponent("Documents")))
    }

    private static func expect(_ condition: @autoclosure () throws -> Bool, _ message: String) throws {
        guard try condition() else { fatalError(message) }
    }

    private static func expectFailure(
        _ expected: DungeonRaidCheckpoints.Failure, _ operation: () throws -> Void
    ) throws {
        do {
            try operation()
            fatalError("Expected \(expected)")
        } catch let actual as DungeonRaidCheckpoints.Failure {
            try expect(actual == expected, "Expected \(expected), got \(actual)")
        }
    }

    private static func writePreferences(_ preferences: [String: Any], to url: URL) throws {
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        let data = try PropertyListSerialization.data(fromPropertyList: preferences, format: .binary, options: 0)
        try data.write(to: url, options: .atomic)
    }

    private static func readPreferences(at url: URL) throws -> [String: Any] {
        let data = try Data(contentsOf: url)
        guard let values = try PropertyListSerialization.propertyList(from: data, options: [], format: nil)
            as? [String: Any] else { fatalError("Expected a property list dictionary") }
        return values
    }
}
