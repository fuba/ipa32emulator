// SPDX-License-Identifier: MIT
// Copyright (c) 2026 IPA32 Emulator contributors

import Foundation

/// Copies the last completed turn from Dungeon Raid 1.3.4's own save data.
struct DungeonRaidCheckpoints {
    enum Slot {
        case number(Int)
        case previous
    }

    enum Failure: LocalizedError, Equatable {
        case noCurrentRun
        case invalidCheckpoint
        case invalidPreferences

        var errorDescription: String? {
            switch self {
            case .noCurrentRun: "Start a run in Dungeon Raid before saving a checkpoint."
            case .invalidCheckpoint: "This checkpoint is empty or damaged."
            case .invalidPreferences: "Dungeon Raid’s preferences could not be read."
            }
        }
    }

    static let slots = 1...3
    private static let bundleID = "com.fireflame.DungeonRaid"
    let preferences: URL
    private let folder: URL

    init(documents: URL) {
        preferences = documents.appendingPathComponent("Saves/\(Self.bundleID)/Library/Preferences/\(Self.bundleID).plist")
        folder = documents.deletingLastPathComponent()
            .appendingPathComponent("Library/Application Support/Dungeon Raid 1.3.4 Checkpoints")
    }

    func url(for slot: Slot) -> URL {
        switch slot {
        case .number(let number): folder.appendingPathComponent("Slot-\(number).plist")
        case .previous: folder.appendingPathComponent("Previous.plist")
        }
    }

    func currentRun() throws -> Data? {
        Self.run(in: try readPreferences())
    }

    func run(in slot: Slot) throws -> Data? {
        guard let data = try Self.readIfPresent(url(for: slot)) else { return nil }
        guard let checkpoint = try? PropertyListDecoder().decode(Checkpoint.self, from: data),
              checkpoint.version == 1, !checkpoint.run.isEmpty
        else {
            throw Failure.invalidCheckpoint
        }
        return checkpoint.run
    }

    func date(in slot: Slot) throws -> Date? {
        let path = url(for: slot)
        do {
            return try path.resourceValues(forKeys: [.contentModificationDateKey])
                .contentModificationDate
        } catch let error as CocoaError where error.code == .fileReadNoSuchFile {
            return nil
        }
    }

    func save(to slot: Int) throws {
        guard let run = try currentRun() else { throw Failure.noCurrentRun }
        try writeCheckpoint(run, to: .number(slot))
    }

    func restore(from slot: Int) throws {
        try restore(.number(slot), preserveCurrent: true)
    }

    func restorePrevious() throws {
        try restore(.previous, preserveCurrent: false)
    }

    private func restore(_ slot: Slot, preserveCurrent: Bool) throws {
        guard let run = try run(in: slot) else { throw Failure.invalidCheckpoint }
        var preferences = try readPreferences()
        let current = Self.run(in: preferences)
        guard current != run else { return }
        if preserveCurrent, let current {
            try writeCheckpoint(current, to: .previous)
        }
        preferences["save"] = run
        let data = try PropertyListSerialization.data(
            fromPropertyList: preferences, format: .binary, options: 0
        )
        try Self.write(data, to: self.preferences)
    }

    private func readPreferences() throws -> [String: Any] {
        guard let data = try Self.readIfPresent(preferences) else { return [:] }
        guard let values = try? PropertyListSerialization.propertyList(
            from: data, options: [], format: nil
        ) as? [String: Any] else {
            throw Failure.invalidPreferences
        }
        return values
    }

    private static func run(in preferences: [String: Any]) -> Data? {
        guard let run = preferences["save"] as? Data, !run.isEmpty else { return nil }
        return run
    }

    private struct Checkpoint: Codable {
        var version = 1
        let run: Data
    }

    private func writeCheckpoint(_ run: Data, to slot: Slot) throws {
        try Self.write(PropertyListEncoder().encode(Checkpoint(run: run)), to: url(for: slot))
    }

    private static func readIfPresent(_ url: URL) throws -> Data? {
        do {
            return try Data(contentsOf: url)
        } catch let error as CocoaError where error.code == .fileReadNoSuchFile {
            return nil
        }
    }

    private static func write(_ data: Data, to url: URL) throws {
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try data.write(to: url, options: .atomic)
    }
}
