// Voice's microphone, speech recognition and speaker, for the GPUI app
// (desktop/gpui/src/voice/helper.rs). Apple's on-device speech recognition
// (SpeechAnalyzer) is Swift only, so it lives in this small program rather
// than in Rust; the rules for what's heard and said are the app's
// (desktop/core/src/voice.rs). It's the Swift app's VoiceAudio and
// VoiceServer (app/Sources/Threadbear/Voice), close to as they were.
//
// It talks JSON, a line at a time: commands on stdin, events on stdout. The
// app's end closing (it quit or crashed) stops everything, pocket-tts too.
// build.rs compiles it, and the app carries it inside its own binary and
// writes it out beside the voice engine the first time Voice is used.
//
// Every time it sends is CACurrentMediaTime's clock, which is the Mac's and
// the same in both programs, so the app can say which came first.
import AVFoundation
import CoreAudio
import CryptoKit
import Darwin
import Foundation
import QuartzCore
import Speech

// MARK: Talking to the app

@MainActor
enum Out {
    static func send(_ event: [String: Any]) {
        guard let data = try? JSONSerialization.data(withJSONObject: event), var line = String(data: data, encoding: .utf8) else { return }
        line += "\n"
        FileHandle.standardOutput.write(Data(line.utf8))
    }

    static func now() -> Double { CACurrentMediaTime() }
}

@main
struct VoiceHelper {
    static func main() async {
        signal(SIGPIPE, SIG_IGN)
        await Helper.shared.run()
    }
}

@MainActor
final class Helper {
    static let shared = Helper()
    private var audio: AnyObject?
    private var previewAudio: AnyObject?
    private var startId = UUID()

    func run() async {
        VoiceServer.reapLeftover()
        Out.send(["event": "hello", "at": Out.now()])
        do {
            for try await line in FileHandle.standardInput.bytes.lines {
                guard let data = line.data(using: .utf8), let cmd = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { continue }
                handle(cmd)
            }
        } catch {}
        // The app's gone: nothing of ours is left running.
        stopListening()
        VoiceServer.shared.stop()
        exit(0)
    }

    private func handle(_ cmd: [String: Any]) {
        switch cmd["cmd"] as? String {
        case "prepare":
            let language = cmd["language"] as? String ?? "english"
            VoiceServer.shared.onStatus = { Out.send(["event": "status", "text": $0]) }
            Task {
                do {
                    _ = try await VoiceServer.shared.ready(language: language)
                    Out.send(["event": "engineReady"])
                } catch is CancellationError {
                } catch {
                    Out.send(["event": "engineFailed", "message": error.localizedDescription])
                }
            }
        case "listen":
            listen(session: cmd["session"] as? Int ?? 0, locale: cmd["locale"] as? String ?? "en-US", microphone: cmd["microphone"] as? String, echo: cmd["echo"] as? Bool ?? true)
        case "stopListening":
            stopListening()
        case "speak":
            guard #available(macOS 26.0, *), let audio = audio as? VoiceAudio else { return }
            let sentences = cmd["sentences"] as? [String] ?? []
            let id = cmd["id"] as? String
            let language = cmd["language"] as? String ?? "english"
            speak(on: audio, sentences: sentences, voice: cmd["voice"] as? String ?? "eve", speed: cmd["speed"] as? Double ?? 1, language: language, id: id)
        case "stopSpeaking":
            if #available(macOS 26.0, *) { (audio as? VoiceAudio)?.stopSpeaking() }
        case "preview":
            guard #available(macOS 26.0, *) else { return }
            preview(text: cmd["text"] as? String ?? "", voice: cmd["voice"] as? String ?? "eve", speed: cmd["speed"] as? Double ?? 1, language: cmd["language"] as? String ?? "english", alsoInUse: audio != nil)
        case "microphones":
            Out.send(["event": "microphones", "list": Microphones.all().map { ["uid": $0.uid, "name": $0.name] }])
        case "stopEngineSoon":
            VoiceServer.shared.stopSoon()
        case "log":
            if let line = cmd["line"] as? String { VoiceLog.write(line) }
        default:
            break
        }
    }

    /// `session` is the app's number for this listening, sent back with
    /// everything it hears, so nothing from an earlier one is taken for it.
    private func listen(session: Int, locale: String, microphone: String?, echo: Bool) {
        stopListening()
        guard #available(macOS 26.0, *) else {
            Out.send(["event": "listenFailed", "session": session, "message": "Voice needs macOS 26 or later."])
            return
        }
        let startId = UUID()
        self.startId = startId
        let audio = VoiceAudio()
        self.audio = audio
        audio.onResult = { r, clockStart in
            Out.send(["event": "heard", "session": session, "text": r.text, "start": r.start, "end": r.end, "final": r.isFinal, "clockStart": clockStart ?? NSNull(), "now": Out.now(), "headphones": VoiceAudio.outputIsHeadphones()])
        }
        audio.onInputLevel = { Out.send(["event": "inputLevel", "level": $0]) }
        audio.onOutputLevel = { Out.send(["event": "outputLevel", "level": $0]) }
        audio.onSpeaking = { Out.send(["event": "speaking", "on": $0, "at": Out.now()]) }
        Task {
            do {
                try await audio.start(locale: Locale(identifier: locale), microphone: Microphones.device(uid: microphone), echoCancellation: echo)
                guard self.startId == startId else { audio.stop(); return }
                Out.send(["event": "listening", "session": session, "echoCancels": audio.echoCancels])
            } catch {
                guard self.startId == startId else { return }
                self.audio = nil
                audio.stop()
                Out.send(["event": "listenFailed", "session": session, "message": error.localizedDescription])
            }
        }
    }

    private func stopListening() {
        startId = UUID()
        if #available(macOS 26.0, *) { (audio as? VoiceAudio)?.stop() }
        audio = nil
    }

    @available(macOS 26.0, *)
    private func speak(on audio: VoiceAudio, sentences: [String], voice: String, speed: Double, language: String, id: String?) {
        Task {
            guard let base = try? await VoiceServer.shared.ready(language: language), self.audio === audio else { return }
            audio.speak(sentences, voice: voice, speed: speed, base: base, played: id.map { id in { Out.send(["event": "played", "id": id, "at": Out.now()]) } })
        }
    }

    @available(macOS 26.0, *)
    private func preview(text: String, voice: String, speed: Double, language: String, alsoInUse: Bool) {
        VoiceServer.shared.onStatus = { Out.send(["event": "previewStatus", "text": $0]) }
        Out.send(["event": "previewStatus", "text": "Getting the voice ready…"])
        Task {
            do {
                let base = try await VoiceServer.shared.ready(language: language)
                let audio = (previewAudio as? VoiceAudio) ?? VoiceAudio()
                previewAudio = audio
                try audio.preview(text, voice: voice, speed: speed, base: base)
                Out.send(["event": "previewStatus", "text": NSNull()])
                // Not out from under a conversation that's using the engine.
                if self.audio == nil { VoiceServer.shared.stopSoon() }
            } catch {
                Out.send(["event": "previewStatus", "text": error.localizedDescription])
            }
        }
    }
}

// MARK: The microphone and the speaker

/// A stretch of what he said, as the recogniser has it.
struct HeardPiece {
    var text: String
    var start: Double
    var end: Double
    var isFinal: Bool
}

/// The microphone and the speaker on one audio engine with the Mac's voice
/// processing on, so its echo cancellation keeps the agent from hearing
/// itself and Mike can talk over it. What he says goes to Apple's on-device
/// speech recognition (SpeechAnalyzer); what the agent says comes from
/// pocket-tts as 24 kHz 16-bit WAV, a sentence at a time.
@available(macOS 26.0, *)
@MainActor
final class VoiceAudio {
    private var engine = AVAudioEngine()
    private var player = AVAudioPlayerNode()
    private var pitch = AVAudioUnitTimePitch()
    /// Some microphone and speaker pairs refuse echo cancellation (Core
    /// Audio's -10875); then Voice runs without it, and the app doesn't
    /// listen while it speaks.
    private(set) var echoCancels = false
    private let speechFormat = AVAudioFormat(commonFormat: .pcmFormatFloat32, sampleRate: 24_000, channels: 1, interleaved: false)!

    private var analyzer: SpeechAnalyzer?
    private var inputContinuation: AsyncStream<AnalyzerInput>.Continuation?
    private var resultsTask: Task<Void, Never>?
    private var speakTask: Task<Void, Never>?
    private var queued = 0
    private var speakId: UUID?
    /// Goes up each time he talks over it or Voice stops: what was fetched or
    /// queued for an earlier round is dropped.
    private var speakRound = 0

    /// A result, and when (CACurrentMediaTime) the audio it counts from began.
    var onResult: ((HeardPiece, Double?) -> Void)?
    var onInputLevel: ((Float) -> Void)?
    var onOutputLevel: ((Float) -> Void)?
    var onSpeaking: ((Bool) -> Void)?

    private(set) var isSpeaking = false
    private let audioClock = AudioClock()

    // MARK: Listening

    /// `echoCancellation` off for listening alone (dictation): nothing is
    /// spoken, so voice processing would only duck other audio.
    func start(locale: Locale, microphone: AudioDeviceID?, echoCancellation: Bool = true) async throws {
        try await Self.askForAccess()
        guard let supported = await SpeechTranscriber.supportedLocale(equivalentTo: locale) else {
            throw VoiceError("Speech recognition doesn't support \(locale.identifier) on this Mac.")
        }
        let transcriber = SpeechTranscriber(locale: supported, transcriptionOptions: [], reportingOptions: [.volatileResults], attributeOptions: [.audioTimeRange])
        if let request = try await AssetInventory.assetInstallationRequest(supporting: [transcriber]) {
            Out.send(["event": "status", "text": "Downloading speech recognition…"])
            try await request.downloadAndInstall()
        }
        let analyzer = SpeechAnalyzer(modules: [transcriber])
        self.analyzer = analyzer
        guard let analyzerFormat = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [transcriber]) else {
            throw VoiceError("Speech recognition has no audio format to use.")
        }
        let (stream, continuation) = AsyncStream<AnalyzerInput>.makeStream()
        inputContinuation = continuation
        let clock = audioClock

        resultsTask = Task { [weak self] in
            do {
                for try await result in transcriber.results {
                    let text = String(result.text.characters)
                    let range = result.range
                    // Its first word's time if it says: a result's range can
                    // begin with the silence before him.
                    let firstWord = result.text.runs.compactMap { $0.audioTimeRange }.first?.start.seconds
                    let piece = HeardPiece(text: text, start: (firstWord ?? range.start.seconds).finite, end: range.end.seconds.finite, isFinal: result.isFinal)
                    self?.onResult?(piece, clock.start)
                }
            } catch {
                VoiceLog.write("recogniser stopped: \(Self.describe(error))")
            }
        }
        try await analyzer.start(inputSequence: stream)

        echoCancels = false
        if echoCancellation {
            do {
                try startEngine(voiceProcessing: true, microphone: microphone, analyzerFormat: analyzerFormat, feed: continuation)
                echoCancels = true
            } catch {
                VoiceLog.write("echo cancellation wouldn't start (\(Self.describe(error))); trying without it")
                resetEngine()
            }
        }
        if !echoCancels {
            do {
                try startEngine(voiceProcessing: false, microphone: microphone, analyzerFormat: analyzerFormat, feed: continuation)
            } catch {
                VoiceLog.write("the microphone wouldn't start: \(Self.describe(error))")
                resetEngine()
                throw VoiceError("The microphone wouldn't start (Core Audio error \((error as NSError).code)). Try another microphone in Settings > Voice.")
            }
        }
        VoiceLog.write("listening, echo cancellation \(echoCancels ? "on" : "off")\(Self.outputIsHeadphones() ? ", headphones" : "")")
    }

    /// A fresh engine: one that failed to start keeps its broken state.
    private func resetEngine() {
        engine.inputNode.removeTap(onBus: 0)
        engine.stop()
        engine = AVAudioEngine()
        player = AVAudioPlayerNode()
        pitch = AVAudioUnitTimePitch()
    }

    private func startEngine(voiceProcessing: Bool, microphone: AudioDeviceID?, analyzerFormat: AVAudioFormat, feed: AsyncStream<AnalyzerInput>.Continuation) throws {
        // Voice processing first, before anything asks the engine for a format.
        if voiceProcessing {
            try engine.inputNode.setVoiceProcessingEnabled(true)
            // Its ducking is for calls; a reply shouldn't hush music any more than it must.
            engine.inputNode.voiceProcessingOtherAudioDuckingConfiguration = .init(enableAdvancedDucking: false, duckingLevel: .min)
        }
        if let microphone { Microphones.use(microphone, on: engine.inputNode) }

        engine.attach(player)
        engine.attach(pitch)
        engine.connect(player, to: pitch, format: speechFormat)
        engine.connect(pitch, to: engine.mainMixerNode, format: speechFormat)

        // No format for the tap: whatever the microphone gives. The converter
        // is made from the first buffer, on the audio thread.
        let converter = ConverterBox(to: analyzerFormat)
        let gate = LevelGate()
        let clock = audioClock
        engine.inputNode.installTap(onBus: 0, bufferSize: 2048, format: nil) { [weak self] buffer, when in
            clock.begin(at: when)
            if let converted = converter.convert(buffer) {
                feed.yield(AnalyzerInput(buffer: converted))
            }
            guard let level = gate.take(Self.level(buffer)) else { return }
            Task { @MainActor in self?.onInputLevel?(level) }
        }
        engine.prepare()
        try engine.start()
        player.play()
    }

    /// Sound goes to headphones (Bluetooth, or the headphone jack), so the
    /// microphone doesn't hear the agent even without echo cancellation.
    nonisolated static func outputIsHeadphones() -> Bool {
        var address = AudioObjectPropertyAddress(mSelector: kAudioHardwarePropertyDefaultOutputDevice, mScope: kAudioObjectPropertyScopeGlobal, mElement: kAudioObjectPropertyElementMain)
        var device = AudioDeviceID(0)
        var size = UInt32(MemoryLayout<AudioDeviceID>.size)
        guard AudioObjectGetPropertyData(AudioObjectID(kAudioObjectSystemObject), &address, 0, nil, &size, &device) == noErr else { return false }
        var transport: UInt32 = 0
        size = UInt32(MemoryLayout<UInt32>.size)
        address.mSelector = kAudioDevicePropertyTransportType
        if AudioObjectGetPropertyData(device, &address, 0, nil, &size, &transport) == noErr,
           transport == kAudioDeviceTransportTypeBluetooth || transport == kAudioDeviceTransportTypeBluetoothLE {
            // With a microphone of its own: a Bluetooth speaker's sound is in
            // the room for this Mac's microphone to hear.
            var input = AudioObjectPropertyAddress(mSelector: kAudioDevicePropertyStreams, mScope: kAudioDevicePropertyScopeInput, mElement: kAudioObjectPropertyElementMain)
            var inputSize: UInt32 = 0
            return AudioObjectGetPropertyDataSize(device, &input, 0, nil, &inputSize) == noErr && inputSize > 0
        }
        var source: UInt32 = 0
        address = AudioObjectPropertyAddress(mSelector: kAudioDevicePropertyDataSource, mScope: kAudioDevicePropertyScopeOutput, mElement: kAudioObjectPropertyElementMain)
        size = UInt32(MemoryLayout<UInt32>.size)
        if AudioObjectGetPropertyData(device, &address, 0, nil, &size, &source) == noErr, source == 0x6864_706E /* 'hdpn' */ {
            return true
        }
        return false
    }

    static func describe(_ error: Error) -> String {
        let e = error as NSError
        return "\(e.domain) \(e.code)"
    }

    func stop() {
        stopSpeaking()
        engine.inputNode.removeTap(onBus: 0)
        engine.stop()
        inputContinuation?.finish()
        inputContinuation = nil
        resultsTask?.cancel()
        resultsTask = nil
        let analyzer = analyzer
        self.analyzer = nil
        Task { await analyzer?.cancelAndFinishNow() }
    }

    // MARK: Speaking

    /// Says the sentences in order, in `voice` at `speed`, from the server at
    /// `base`. `played` runs once the last of them has come out of the
    /// speakers, and never if any was cut off (he talked over it, it failed).
    func speak(_ sentences: [String], voice: String, speed: Double, base: URL, played: (() -> Void)? = nil) {
        guard !sentences.isEmpty else { return }
        pitch.rate = Float(speed)
        let previous = speakTask
        let id = UUID()
        let round = speakRound
        speakId = id
        speakTask = Task { [weak self] in
            await previous?.value
            var whole = true
            for sentence in sentences {
                guard let self, !Task.isCancelled, self.speakRound == round else { return }
                do {
                    let samples = try await self.stream(sentence, voice: voice, base: base, round: round)
                    // A 200 with little or no sound in it isn't that sentence said.
                    if samples < Self.fewestSamples(for: sentence) { whole = false }
                } catch {
                    if !Task.isCancelled { VoiceLog.write("couldn't speak: \(error.localizedDescription)") }
                    whole = false
                    break
                }
            }
            guard let self else { return }
            if whole, let played, !Task.isCancelled, self.speakRound == round { self.markPlayed(round: round, played) }
            guard self.speakId == id else { return }
            self.speakTask = nil
            if self.queued == 0 { self.setSpeaking(false) }
        }
    }

    /// Stops talking now: he's started speaking, or Voice was turned off.
    func stopSpeaking() {
        speakRound += 1
        speakTask?.cancel()
        speakTask = nil
        player.stop()
        if engine.isRunning { player.play() }
        queued = 0
        setSpeaking(false)
    }

    /// Far fewer than any voice takes over it (40 ms a letter), to catch a
    /// reply that came back with little in it.
    static func fewestSamples(for sentence: String) -> Int {
        sentence.count * 960
    }

    @discardableResult
    private func stream(_ text: String, voice: String, base: URL, round: Int) async throws -> Int {
        var request = URLRequest(url: base.appendingPathComponent("tts"))
        request.httpMethod = "POST"
        let boundary = "tb-\(UUID().uuidString)"
        request.setValue("multipart/form-data; boundary=\(boundary)", forHTTPHeaderField: "Content-Type")
        request.httpBody = Self.form(["text": text, "voice_url": voice], boundary: boundary)
        var samples = 0
        for try await piece in Self.speech(request) {
            try Task.checkCancellation()
            guard speakRound == round else { return samples }
            schedule(piece, round: round)
            samples += piece.samples.count
        }
        return samples
    }

    struct SpeechPiece: Sendable {
        let samples: [Float]
        let peak: Float
    }

    /// The reply as it arrives, read and decoded off the main thread, which
    /// only gets a finished piece about ten times a second.
    private nonisolated static func speech(_ request: URLRequest) -> AsyncThrowingStream<SpeechPiece, Error> {
        AsyncThrowingStream { continuation in
            let task = Task.detached(priority: .userInitiated) {
                do {
                    let (bytes, response) = try await URLSession.shared.bytes(for: request)
                    let status = (response as? HTTPURLResponse)?.statusCode ?? 0
                    guard status == 200 else { throw VoiceError("the voice engine said \(status)") }
                    var wav = WAVStream()
                    var chunk = Data()
                    chunk.reserveCapacity(4800)
                    for try await byte in bytes {
                        try Task.checkCancellation()
                        chunk.append(byte)
                        if chunk.count >= 4800 {
                            continuation.yield(Self.piece(wav.samples(from: chunk)))
                            chunk.removeAll(keepingCapacity: true)
                        }
                    }
                    if !chunk.isEmpty { continuation.yield(Self.piece(wav.samples(from: chunk))) }
                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }

    private nonisolated static func piece(_ samples: [Int16]) -> SpeechPiece {
        var peak: Float = 0
        let floats = samples.map { sample -> Float in
            let value = Float(sample) / 32768
            peak = max(peak, abs(value))
            return value
        }
        return SpeechPiece(samples: floats, peak: peak)
    }

    private func schedule(_ piece: SpeechPiece, round: Int) {
        let count = piece.samples.count
        guard count > 0, let buffer = AVAudioPCMBuffer(pcmFormat: speechFormat, frameCapacity: AVAudioFrameCount(count)) else { return }
        buffer.frameLength = AVAudioFrameCount(count)
        piece.samples.withUnsafeBufferPointer { buffer.floatChannelData![0].update(from: $0.baseAddress!, count: count) }
        let level = min(1, piece.peak * 1.4)
        queued += 1
        setSpeaking(true)
        onOutputLevel?(level)
        // Played back, not just taken: his words can't be an answer while
        // it's still coming out of a speaker.
        player.scheduleBuffer(buffer, completionCallbackType: .dataPlayedBack) { [weak self] _ in
            Task { @MainActor in
                guard let self, self.speakRound == round else { return }
                self.queued = max(0, self.queued - 1)
                self.onOutputLevel?(self.queued == 0 ? 0 : level)
                if self.queued == 0, self.speakTask == nil { self.setSpeaking(false) }
            }
        }
        if !player.isPlaying { player.play() }
    }

    /// `played` once all that's queued now has really been heard: a moment of
    /// silence after it, with the played-back callback.
    private func markPlayed(round: Int, _ played: @escaping () -> Void) {
        guard let silence = AVAudioPCMBuffer(pcmFormat: speechFormat, frameCapacity: 240) else { return }
        silence.frameLength = 240
        silence.floatChannelData![0].update(repeating: 0, count: 240)
        player.scheduleBuffer(silence, completionCallbackType: .dataPlayedBack) { [weak self] _ in
            Task { @MainActor in
                guard let self, self.speakRound == round else { return }
                played()
            }
        }
    }

    private func setSpeaking(_ on: Bool) {
        guard isSpeaking != on else { return }
        isSpeaking = on
        onSpeaking?(on)
    }

    /// Plays a short sample in the given voice (Settings' Play a Sample).
    func preview(_ text: String, voice: String, speed: Double, base: URL) throws {
        if !engine.isRunning {
            engine.attach(player)
            engine.attach(pitch)
            engine.connect(player, to: pitch, format: speechFormat)
            engine.connect(pitch, to: engine.mainMixerNode, format: speechFormat)
            try engine.start()
            player.play()
        }
        stopSpeaking()
        speak([text], voice: voice, speed: speed, base: base)
    }

    static func form(_ fields: [String: String], boundary: String) -> Data {
        var body = Data()
        for (name, value) in fields.sorted(by: { $0.key < $1.key }) {
            body.append(Data("--\(boundary)\r\nContent-Disposition: form-data; name=\"\(name)\"\r\n\r\n\(value)\r\n".utf8))
        }
        body.append(Data("--\(boundary)--\r\n".utf8))
        return body
    }

    nonisolated static func level(_ buffer: AVAudioPCMBuffer) -> Float {
        guard let data = buffer.floatChannelData?[0], buffer.frameLength > 0 else { return 0 }
        var sum: Float = 0
        for i in 0..<Int(buffer.frameLength) { sum += data[i] * data[i] }
        let rms = (sum / Float(buffer.frameLength)).squareRoot()
        return min(1, rms * 12)
    }

    static func askForAccess() async throws {
        switch AVCaptureDevice.authorizationStatus(for: .audio) {
        case .authorized: break
        case .notDetermined:
            guard await AVCaptureDevice.requestAccess(for: .audio) else { throw VoiceError("Threadbear isn't allowed to use the microphone.") }
        default:
            throw VoiceError("Threadbear isn't allowed to use the microphone. Turn it on in System Settings > Privacy & Security > Microphone.")
        }
    }
}

// MARK: Microphones

enum Microphones {
    struct Microphone {
        let id: AudioDeviceID
        let uid: String
        let name: String
    }

    /// Every input device, by name.
    static func all() -> [Microphone] {
        var address = AudioObjectPropertyAddress(mSelector: kAudioHardwarePropertyDevices, mScope: kAudioObjectPropertyScopeGlobal, mElement: kAudioObjectPropertyElementMain)
        var size: UInt32 = 0
        guard AudioObjectGetPropertyDataSize(AudioObjectID(kAudioObjectSystemObject), &address, 0, nil, &size) == noErr else { return [] }
        var ids = [AudioDeviceID](repeating: 0, count: Int(size) / MemoryLayout<AudioDeviceID>.size)
        guard AudioObjectGetPropertyData(AudioObjectID(kAudioObjectSystemObject), &address, 0, nil, &size, &ids) == noErr else { return [] }
        return ids.compactMap { id in
            var streams = AudioObjectPropertyAddress(mSelector: kAudioDevicePropertyStreams, mScope: kAudioDevicePropertyScopeInput, mElement: kAudioObjectPropertyElementMain)
            var streamSize: UInt32 = 0
            guard AudioObjectGetPropertyDataSize(id, &streams, 0, nil, &streamSize) == noErr, streamSize > 0 else { return nil }
            guard let name = string(id, kAudioObjectPropertyName), let uid = string(id, kAudioDevicePropertyDeviceUID) else { return nil }
            return Microphone(id: id, uid: uid, name: name)
        }
    }

    static func device(uid: String?) -> AudioDeviceID? {
        guard let uid, !uid.isEmpty else { return nil }
        return all().first { $0.uid == uid }?.id
    }

    private static func string(_ id: AudioObjectID, _ selector: AudioObjectPropertySelector) -> String? {
        var address = AudioObjectPropertyAddress(mSelector: selector, mScope: kAudioObjectPropertyScopeGlobal, mElement: kAudioObjectPropertyElementMain)
        var value: Unmanaged<CFString>?
        var size = UInt32(MemoryLayout<Unmanaged<CFString>?>.size)
        guard AudioObjectGetPropertyData(id, &address, 0, nil, &size, &value) == noErr, let value else { return nil }
        return value.takeRetainedValue() as String
    }

    static func use(_ device: AudioDeviceID, on input: AVAudioInputNode) {
        guard let unit = input.audioUnit else { return }
        var device = device
        let status = AudioUnitSetProperty(unit, kAudioOutputUnitProperty_CurrentDevice, kAudioUnitScope_Global, 0, &device, UInt32(MemoryLayout<AudioDeviceID>.size))
        if status != noErr { VoiceLog.write("couldn't pick that microphone (\(status)); using the default") }
    }
}

/// When the microphone's first buffer was captured, from the audio thread.
/// Only ever the first: a later one's time would make everything heard look
/// more recent than it was. A first with no time leaves it unknown, and an
/// unknown time can't answer a card.
final class AudioClock: @unchecked Sendable {
    private let lock = NSLock()
    private var began = false
    private var first: Double?

    func begin(at when: AVAudioTime) {
        lock.lock()
        defer { lock.unlock() }
        guard !began else { return }
        began = true
        if when.isHostTimeValid { first = AVAudioTime.seconds(forHostTime: when.hostTime) }
    }

    var start: Double? {
        lock.lock()
        defer { lock.unlock() }
        return first
    }
}

/// Lets a level through about twelve times a second, the loudest since the last.
final class LevelGate: @unchecked Sendable {
    private var last: CFTimeInterval = 0
    private var loudest: Float = 0

    func take(_ level: Float) -> Float? {
        loudest = max(loudest, level)
        let now = CACurrentMediaTime()
        guard now - last >= 0.08 else { return nil }
        defer { last = now; loudest = 0 }
        return loudest
    }
}

/// Makes the converter from the microphone's first buffer and converts each
/// one to what speech recognition wants. Only the audio thread uses it.
final class ConverterBox: @unchecked Sendable {
    private let format: AVAudioFormat
    private var converter: AVAudioConverter?
    private var failed = false

    init(to format: AVAudioFormat) { self.format = format }

    func convert(_ buffer: AVAudioPCMBuffer) -> AVAudioPCMBuffer? {
        if converter == nil, !failed {
            converter = AVAudioConverter(from: buffer.format, to: format)
            if buffer.format.channelCount > format.channelCount { converter?.channelMap = [0] }
            if converter == nil {
                failed = true
                VoiceLog.write("can't convert \(buffer.format) for speech recognition")
            }
        }
        guard let converter else { return nil }
        let ratio = format.sampleRate / buffer.format.sampleRate
        let capacity = AVAudioFrameCount(Double(buffer.frameLength) * ratio + 16)
        guard let out = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: capacity) else { return nil }
        var fed = false
        var error: NSError?
        converter.convert(to: out, error: &error) { _, status in
            if fed { status.pointee = .noDataNow; return nil }
            fed = true
            status.pointee = .haveData
            return buffer
        }
        return error == nil && out.frameLength > 0 ? out : nil
    }
}

/// Voice's own log, ~/Library/Logs/Threadbear/voice.log, so what went wrong
/// can be read afterwards. Kept under 256 KB. Never what was said.
enum VoiceLog {
    static let url = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Logs/Threadbear/voice.log")
    private static let queue = DispatchQueue(label: "voice-log")

    static func write(_ line: String) {
        let stamp = ISO8601DateFormatter().string(from: Date())
        queue.async {
            let fm = FileManager.default
            try? fm.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            if let size = (try? fm.attributesOfItem(atPath: url.path))?[.size] as? Int, size > 256_000 { try? fm.removeItem(at: url) }
            if !fm.fileExists(atPath: url.path) { fm.createFile(atPath: url.path, contents: nil) }
            guard let handle = try? FileHandle(forWritingTo: url) else { return }
            handle.seekToEndOfFile()
            handle.write(Data("\(stamp) [gpui] \(line)\n".utf8))
            try? handle.close()
        }
    }
}

struct VoiceError: LocalizedError {
    let message: String
    init(_ message: String) { self.message = message }
    var errorDescription: String? { message }
}

private extension Double {
    var finite: Double { isFinite ? self : 0 }
}

/// 16-bit PCM out of a WAV that arrives in pieces: skips the header (its
/// chunks, up to "data"), then hands back whole samples, keeping an odd byte
/// for the next piece.
struct WAVStream {
    private var header = Data()
    private var inData = false
    private var carry: UInt8?

    mutating func samples(from piece: Data) -> [Int16] {
        var bytes = piece
        if !inData {
            header.append(bytes)
            guard let start = Self.dataStart(header) else { return [] }
            inData = true
            bytes = header.subdata(in: start..<header.count)
            header = Data()
        }
        var all = [UInt8]()
        all.reserveCapacity(bytes.count + 1)
        if let carry { all.append(carry) }
        all.append(contentsOf: bytes)
        carry = all.count % 2 == 1 ? all.removeLast() : nil
        var out = [Int16](repeating: 0, count: all.count / 2)
        for i in 0..<out.count { out[i] = Int16(bitPattern: UInt16(all[2 * i]) | UInt16(all[2 * i + 1]) << 8) }
        return out
    }

    static func dataStart(_ header: Data) -> Int? {
        let b = [UInt8](header)
        guard b.count >= 12 else { return nil }
        var i = 12
        while i + 8 <= b.count {
            let id = String(bytes: b[i..<i + 4], encoding: .ascii)
            let size = Int(b[i + 4]) | Int(b[i + 5]) << 8 | Int(b[i + 6]) << 16 | Int(b[i + 7]) << 24
            if id == "data" { return i + 8 }
            i += 8 + size
        }
        return nil
    }
}

// MARK: pocket-tts

/// Kyutai's pocket-tts, which gives Voice its voice: a small program that
/// runs on the CPU. The first time Voice is used it's downloaded from its
/// GitHub release (that exact version, checked against its checksum) into
/// ~/Library/Application Support/Threadbear/voice/, the same folder and
/// version the Swift app used, and run as `pocket-tts serve` on a free port
/// on 127.0.0.1 while Voice is in use (it stops five minutes after, and when
/// the app quits; after a crash, the next start stops it, from its pid
/// file). Its model and voices come from Hugging Face on first use, into
/// ~/.cache/huggingface (about 240 MB). docs/on-your-mac.md lists all of it.
@MainActor
final class VoiceServer {
    static let shared = VoiceServer()

    enum Failure: LocalizedError {
        case notAppleSilicon, download(String), checksum, start(String)
        var errorDescription: String? {
            switch self {
            case .notAppleSilicon: "Voice needs a Mac with Apple silicon."
            case .download(let why): "Couldn't download the voice engine: \(why)"
            case .checksum: "The voice engine download didn't match its checksum, so it wasn't used."
            case .start(let why): "The voice engine didn't start: \(why)"
            }
        }
    }

    nonisolated static let release = "v4.0.1"
    static let asset = "pocket-tts-3.3.0-aarch64-apple-darwin.tar.gz"
    nonisolated static let sha256 = "ee9ca6c7c000ee72c5846e3a8ba01d08f56636d94a614d2259c3f12b8fd69afa"
    static var downloadURL: URL { URL(string: "https://github.com/kyutai-labs/pocket-tts/releases/download/\(release)/\(asset)")! }

    nonisolated static var folder: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Threadbear/voice", isDirectory: true)
    }
    nonisolated static var binary: URL { folder.appendingPathComponent("\(release)/pocket-tts") }

    private var process: Process?
    private var port: Int?
    private var language: String?
    /// The start under way. A newer one replaces it, and it gives up.
    private var starting: (id: UUID, task: Task<URL, Error>)?
    private var idleStop: Task<Void, Never>?
    var onStatus: ((String) -> Void)?

    var isInstalled: Bool { FileManager.default.isExecutableFile(atPath: Self.binary.path) }

    func ready(language: String) async throws -> URL {
        idleStop?.cancel()
        if let process, process.isRunning, let port, self.language == language {
            return URL(string: "http://127.0.0.1:\(port)")!
        }
        if let starting, self.language == language { return try await starting.task.value }
        stop()
        self.language = language
        let id = UUID()
        let task = Task { try await self.start(language: language, id: id) }
        starting = (id, task)
        defer { if starting?.id == id { starting = nil } }
        return try await task.value
    }

    func stopSoon() {
        idleStop?.cancel()
        idleStop = Task { [weak self] in
            try? await Task.sleep(for: .seconds(300))
            guard !Task.isCancelled else { return }
            self?.stop()
        }
    }

    func stop() {
        idleStop?.cancel()
        starting?.task.cancel()
        starting = nil
        if let process, process.isRunning { process.terminate() }
        process = nil
        port = nil
        language = nil
        try? FileManager.default.removeItem(at: Self.pidFile(owner: getpid()))
    }

    /// The running server's process id, in a file named for the program that
    /// started it: the Swift app and this helper share the folder.
    nonisolated private static func pidFile(owner: pid_t) -> URL { folder.appendingPathComponent("serve.\(owner).pid") }

    /// Stops a server an earlier run left going, checking it's really ours
    /// first: pids get reused. One whose owner is still running is left alone.
    nonisolated static func reapLeftover() {
        let files = (try? FileManager.default.contentsOfDirectory(atPath: folder.path)) ?? []
        for name in files where name.hasPrefix("serve.") && name.hasSuffix(".pid") {
            let file = folder.appendingPathComponent(name)
            guard let owner = pid_t(name.dropFirst("serve.".count).dropLast(".pid".count)) else { continue }
            if owner != getpid(), kill(owner, 0) == 0 { continue }
            if let text = try? String(contentsOf: file, encoding: .utf8),
               let pid = pid_t(text.trimmingCharacters(in: .whitespacesAndNewlines)), pid > 1,
               executablePath(of: pid) == binary.standardizedFileURL.path {
                kill(pid, SIGTERM)
                VoiceLog.write("stopped pocket-tts left over from an earlier run")
            }
            try? FileManager.default.removeItem(at: file)
        }
    }

    nonisolated private static func executablePath(of pid: pid_t) -> String? {
        var buffer = [CChar](repeating: 0, count: 4 * Int(MAXPATHLEN))
        guard proc_pidpath(pid, &buffer, UInt32(buffer.count)) > 0 else { return nil }
        return String(cString: buffer)
    }

    private func start(language: String, id: UUID) async throws -> URL {
        #if !arch(arm64)
        throw Failure.notAppleSilicon
        #else
        if !isInstalled { try await install(id: id) }
        guard starting?.id == id, !Task.isCancelled else { throw CancellationError() }
        Self.reapLeftover()
        onStatus?("Starting the voice engine…")
        let port = try Self.freePort()
        let process = Process()
        process.executableURL = Self.binary
        process.arguments = ["serve", "--host", "127.0.0.1", "--port", String(port), "--language", language]
        process.currentDirectoryURL = Self.folder
        let log = Self.folder.appendingPathComponent("serve.log")
        FileManager.default.createFile(atPath: log.path, contents: nil)
        let handle = try FileHandle(forWritingTo: log)
        process.standardOutput = handle
        process.standardError = handle
        do { try process.run() } catch { throw Failure.start(error.localizedDescription) }
        self.process = process
        self.port = port
        try? String(process.processIdentifier).write(to: Self.pidFile(owner: getpid()), atomically: true, encoding: .utf8)
        let base = URL(string: "http://127.0.0.1:\(port)")!
        onStatus?(FileManager.default.fileExists(atPath: Self.hubCache.path) ? "Starting the voice engine…" : "Downloading the voice model (about 240 MB)…")
        for _ in 0..<600 {
            guard starting?.id == id, !Task.isCancelled else {
                if process.isRunning { process.terminate() }
                throw CancellationError()
            }
            guard process.isRunning else {
                let tail = (try? String(contentsOf: log, encoding: .utf8))?.split(separator: "\n").suffix(2).joined(separator: " ") ?? ""
                throw Failure.start(tail.isEmpty ? "it stopped" : tail)
            }
            if let (_, response) = try? await URLSession.shared.data(from: base), (response as? HTTPURLResponse)?.statusCode == 200 {
                VoiceLog.write("pocket-tts \(Self.release) up on \(port) (\(language))")
                return base
            }
            try await Task.sleep(for: .milliseconds(500))
        }
        if starting?.id == id { stop() } else if process.isRunning { process.terminate() }
        throw Failure.start("it took too long")
        #endif
    }

    static var hubCache: URL {
        FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".cache/huggingface/hub/models--kyutai--pocket-tts-without-voice-cloning")
    }

    private func install(id: UUID) async throws {
        onStatus?("Downloading the voice engine…")
        let (file, response): (URL, URLResponse)
        do { (file, response) = try await URLSession.shared.download(from: Self.downloadURL) } catch { throw Failure.download(error.localizedDescription) }
        defer { try? FileManager.default.removeItem(at: file) }
        guard (response as? HTTPURLResponse)?.statusCode == 200 else { throw Failure.download("the server said \((response as? HTTPURLResponse)?.statusCode ?? 0)") }
        let unpack = Self.folder.appendingPathComponent("unpack-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: unpack) }
        let inner = try await Task.detached(priority: .userInitiated) {
            try Self.checkAndUnpack(file, into: unpack)
        }.value
        guard starting?.id == id else { throw CancellationError() }
        let dest = Self.binary.deletingLastPathComponent()
        try? FileManager.default.removeItem(at: dest)
        try FileManager.default.createDirectory(at: dest.deletingLastPathComponent(), withIntermediateDirectories: true)
        try FileManager.default.moveItem(at: inner, to: dest)
        VoiceLog.write("installed pocket-tts \(Self.release)")
    }

    /// The archive's checksum, then its contents into `unpack`. Nothing is
    /// unpacked from one that doesn't match.
    private nonisolated static func checkAndUnpack(_ file: URL, into unpack: URL) throws -> URL {
        let data = try Data(contentsOf: file, options: .mappedIfSafe)
        guard SHA256.hash(data: data).map({ String(format: "%02x", $0) }).joined() == sha256 else { throw Failure.checksum }
        try FileManager.default.createDirectory(at: unpack, withIntermediateDirectories: true)
        let tar = Process()
        tar.executableURL = URL(fileURLWithPath: "/usr/bin/tar")
        tar.arguments = ["-xzf", file.path, "-C", unpack.path]
        try tar.run()
        tar.waitUntilExit()
        guard tar.terminationStatus == 0,
              let inner = try FileManager.default.contentsOfDirectory(at: unpack, includingPropertiesForKeys: nil).first(where: { $0.hasDirectoryPath }),
              FileManager.default.fileExists(atPath: inner.appendingPathComponent("pocket-tts").path)
        else { throw Failure.download("the archive wasn't as expected") }
        return inner
    }

    static func freePort() throws -> Int {
        let fd = socket(AF_INET, SOCK_STREAM, 0)
        guard fd >= 0 else { throw Failure.start("no socket") }
        defer { close(fd) }
        var addr = sockaddr_in()
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_addr.s_addr = inet_addr("127.0.0.1")
        addr.sin_port = 0
        var len = socklen_t(MemoryLayout<sockaddr_in>.size)
        let bound = withUnsafeMutablePointer(to: &addr) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) { bind(fd, $0, len) == 0 && getsockname(fd, $0, &len) == 0 }
        }
        guard bound else { throw Failure.start("no free port") }
        return Int(UInt16(bigEndian: addr.sin_port))
    }
}
