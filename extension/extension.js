// extension.js
// VS Code extension that starts tsn-lsp over stdio and provides run/debug
// capabilities for .tsn files via tsn.
// CommonJS module — no build step required.

"use strict";

const vscode  = require("vscode");
const { LanguageClient, TransportKind, State } = require("vscode-languageclient/node");
const path    = require("path");
const fs      = require("fs");
const { spawn } = require("child_process");

/** @type {LanguageClient | undefined} */
let client;

/** @type {vscode.StatusBarItem | undefined} */
let statusBar;

/** @type {vscode.OutputChannel | undefined} */
let runChannel;

/** @type {vscode.OutputChannel | undefined} */
let lspOutputChannel;

// ─────────────────────────────────────────────────────────────────────────────
// Activation / deactivation
// ─────────────────────────────────────────────────────────────────────────────

/**
 * @param {vscode.ExtensionContext} context
 */
async function activate(context) {
    // ── Status bar ───────────────────────────────────────────────────────────
    statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 10);
    statusBar.command = "tsn.restartServer";
    setStatus("starting");
    statusBar.show();
    context.subscriptions.push(statusBar);

    // ── Language Server ──────────────────────────────────────────────────────
    lspOutputChannel = vscode.window.createOutputChannel("TSN Language Server");
    const lspPath = resolveLspPath(context);
    if (!lspPath) {
        setStatus("error");
        vscode.window.showErrorMessage(
            "tsn-lsp binary not found. " +
            "Build with `cargo build --release --bin tsn-lsp` or set tsn.server.path."
        );
    } else {
        client = createClient(lspPath, lspOutputChannel);
        client.onDidChangeState(({ newState }) => {
            if      (newState === State.Running)  setStatus("running");
            else if (newState === State.Starting) setStatus("starting");
            else                                  setStatus("stopped");
        });
    }

    // ── Debug adapter ────────────────────────────────────────────────────────
    const factory = new TsnDebugAdapterFactory(context);
    context.subscriptions.push(
        vscode.debug.registerDebugAdapterDescriptorFactory("tsn", factory),
        vscode.debug.registerDebugConfigurationProvider("tsn", new TsnDebugConfigProvider())
    );

    // ── Commands ─────────────────────────────────────────────────────────────
    context.subscriptions.push(
        registerTsnCodeLensProvider(),

        vscode.commands.registerCommand("tsn.restartServer", async () => {
            await stopClient();
            if (!lspPath) return;
            setStatus("starting");
            client = createClient(lspPath, lspOutputChannel);
            client.onDidChangeState(({ newState }) => {
                if      (newState === State.Running)  setStatus("running");
                else if (newState === State.Starting) setStatus("starting");
                else                                  setStatus("stopped");
            });
            await client.start();
        }),

        vscode.commands.registerCommand("tsn.stopServer", async () => {
            await stopClient();
            setStatus("stopped");
        }),

        vscode.commands.registerCommand("tsn.showServerLog", () => {
            client?.outputChannel.show();
        }),

        vscode.commands.registerCommand("tsn.runFile", (uri) => {
            runTsnFile(context, uri, { terminal: false });
        }),

        vscode.commands.registerCommand("tsn.runFileInTerminal", (uri) => {
            runTsnFile(context, uri, { terminal: true });
        }),

        vscode.commands.registerCommand("tsn.showAst", (uri) => {
            runWithDebugPhase(context, uri, ["ast"], "AST");
        }),

        vscode.commands.registerCommand("tsn.showTokens", (uri) => {
            runWithDebugPhase(context, uri, ["tokens"], "Tokens");
        }),

        vscode.commands.registerCommand("tsn.disasmFile", (uri) => {
            runDisasm(context, uri);
        }),

        vscode.commands.registerCommand("tsn.benchFile", (uri) => {
            runBench(context, uri);
        }),

        vscode.commands.registerCommand("tsn.doctor", () => {
            runDoctor(context);
        }),

        vscode.commands.registerCommand("tsn.installRuntime", () => {
            runInstaller(context);
        }),

        { dispose: () => stopClient() }
    );

    // Start LSP after commands are registered so a startup failure
    // never prevents command registration.
    if (client) {
        await client.start().catch((err) => {
            setStatus("stopped");
            vscode.window.showErrorMessage(`TSN Language Server failed to start: ${err?.message ?? err}`);
        });
    }
}

async function deactivate() {
    await stopClient();
}

module.exports = { activate, deactivate };

// ─────────────────────────────────────────────────────────────────────────────
// Status bar helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * @param {"starting"|"running"|"stopped"|"error"} state
 */
function setStatus(state) {
    if (!statusBar) return;
    switch (state) {
        case "starting":
            statusBar.text    = "$(loading~spin) TSN";
            statusBar.tooltip = "TSN Language Server — Starting";
            statusBar.backgroundColor = undefined;
            break;
        case "running":
            statusBar.text    = "$(check) TSN";
            statusBar.tooltip = "TSN Language Server — Running  (click to restart)";
            statusBar.backgroundColor = undefined;
            break;
        case "stopped":
            statusBar.text    = "$(circle-slash) TSN";
            statusBar.tooltip = "TSN Language Server — Stopped  (click to restart)";
            statusBar.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
            break;
        case "error":
            statusBar.text    = "$(error) TSN";
            statusBar.tooltip = "TSN Language Server — Binary not found";
            statusBar.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
            break;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Run commands
// ─────────────────────────────────────────────────────────────────────────────

/**
 * @param {vscode.ExtensionContext} context
 * @param {vscode.Uri | undefined} uri
 * @param {{ terminal: boolean, verbose?: boolean, debugPhases?: string[], noRun?: boolean }} opts
 */
function runTsnFile(context, uri, opts) {
    const file = resolveTargetFile(uri);
    if (!file) return;

    const cliPath = resolveCliPath(context);
    if (!cliPath) {
        vscode.window.showErrorMessage(
            "tsn binary not found. " +
            "Build with `cargo build --release --bin tsn` or set tsn.cli.path."
        );
        return;
    }

    const cfg = vscode.workspace.getConfiguration("tsn");
    const verbose = opts.verbose ?? cfg.get("run.verbose") ?? false;
    const phases  = opts.debugPhases ?? cfg.get("run.debugPhases") ?? [];
    const noRun   = opts.noRun ?? false;

    const args = buildCliArgs(file, { verbose, debugPhases: phases, noRun });

    if (opts.terminal) {
        const t = vscode.window.createTerminal({
            name: `TSN: ${path.basename(file)}`,
            cwd:  path.dirname(file),
        });
        t.show(true);
        t.sendText(`"${cliPath}" ${args.map(a => `"${a}"`).join(" ")}`);
    } else {
        runInOutputChannel(cliPath, args, path.dirname(file), path.basename(file));
    }
}

/**
 * @param {vscode.ExtensionContext} context
 * @param {vscode.Uri | undefined} uri
 * @param {string[]} phases
 * @param {string} label
 */
function runWithDebugPhase(context, uri, phases, label) {
    const file = resolveTargetFile(uri);
    if (!file) return;

    const cliPath = resolveCliPath(context);
    if (!cliPath) {
        vscode.window.showErrorMessage("tsn binary not found.");
        return;
    }

    const args = buildCliArgs(file, { verbose: false, debugPhases: phases, noRun: true });
    runInOutputChannel(cliPath, args, path.dirname(file), `${label}: ${path.basename(file)}`);
}

/**
 * @param {vscode.ExtensionContext} context
 * @param {vscode.Uri | undefined} uri
 */
function runDisasm(context, uri) {
    const file = resolveTargetFile(uri);
    if (!file) return;

    const cliPath = resolveCliPath(context);
    if (!cliPath) {
        vscode.window.showErrorMessage("tsn binary not found.");
        return;
    }

    runInOutputChannel(cliPath, ["disasm", file], path.dirname(file), `Disasm: ${path.basename(file)}`);
}

/**
 * @param {vscode.ExtensionContext} context
 * @param {vscode.Uri | undefined} uri
 */
function runBench(context, uri) {
    const file = resolveTargetFile(uri);
    if (!file) return;

    const cliPath = resolveCliPath(context);
    if (!cliPath) {
        vscode.window.showErrorMessage("tsn binary not found.");
        return;
    }

    runInOutputChannel(cliPath, ["bench", file], path.dirname(file), `Bench: ${path.basename(file)}`);
}

/**
 * @param {vscode.ExtensionContext} context
 */
function runDoctor(context) {
    const cliPath = resolveCliPath(context);
    if (!cliPath) {
        vscode.window.showErrorMessage("tsn binary not found.");
        return;
    }
    const cwd = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath ?? context.extensionPath;
    runInOutputChannel(cliPath, ["doctor"], cwd, "TSN Doctor");
}

/**
 * @param {vscode.ExtensionContext} context
 */
function runInstaller(context) {
    const root = path.resolve(context.extensionPath, "..");
    const isWin = process.platform === "win32";
    const script = isWin
        ? path.join(root, "scripts", "install.ps1")
        : path.join(root, "scripts", "install.sh");

    if (!fs.existsSync(script)) {
        vscode.window.showErrorMessage(`TSN installer script not found: ${script}`);
        return;
    }

    const t = vscode.window.createTerminal({
        name: "TSN Install Runtime",
        cwd: root,
    });
    t.show(true);
    if (isWin) {
        t.sendText(`powershell -ExecutionPolicy Bypass -File "${script}"`);
    } else {
        t.sendText(`chmod +x "${script}" && "${script}"`);
    }
}

/**
 * Build tsn arg list for a `run` invocation.
 * @param {string} file
 * @param {{ verbose: boolean, debugPhases: string[], noRun: boolean }} opts
 * @returns {string[]}
 */
function buildCliArgs(file, opts) {
    const args = [file];
    if (opts.verbose) args.push("--verbose");
    if (opts.noRun)   args.push("--noRun");
    if (opts.debugPhases && opts.debugPhases.length > 0) {
        args.push(`--debug=${opts.debugPhases.join(",")}`);
    }
    return args;
}

/**
 * Resolve the active .tsn file path from a URI or the active editor.
 * @param {vscode.Uri | undefined} uri
 * @returns {string | undefined}
 */
function resolveTargetFile(uri) {
    const fsPath = uri?.fsPath ?? vscode.window.activeTextEditor?.document.fileName;
    if (!fsPath || !fsPath.endsWith(".tsn")) {
        vscode.window.showWarningMessage("No TSN file to run. Open a .tsn file first.");
        return undefined;
    }
    return fsPath;
}

/**
 * Run a process and stream its output to the shared TSN Output channel.
 * @param {string} bin
 * @param {string[]} args
 * @param {string} cwd
 * @param {string} label
 */
function runInOutputChannel(bin, args, cwd, label) {
    if (!runChannel) {
        runChannel = vscode.window.createOutputChannel("TSN Output");
    }
    runChannel.clear();
    runChannel.show(true);
    runChannel.appendLine(`▶  ${label}`);
    runChannel.appendLine("─".repeat(64));

    const proc = spawn(bin, args, { cwd });

    proc.stdout.on("data", (d) => runChannel.append(d.toString()));
    proc.stderr.on("data", (d) => runChannel.append(d.toString()));
    proc.on("error", (err) => {
        runChannel.appendLine(`\nError: ${err.message}`);
    });
    proc.on("close", (code) => {
        runChannel.appendLine("─".repeat(64));
        runChannel.appendLine(`Exited with code ${code}`);
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Debug adapter
// ─────────────────────────────────────────────────────────────────────────────

class TsnDebugAdapterFactory {
    /** @param {vscode.ExtensionContext} context */
    constructor(context) { this._context = context; }

    /** @param {vscode.DebugSession} session */
    createDebugAdapterDescriptor(session) {
        const cliPath = resolveCliPath(this._context);
        if (!cliPath) {
            vscode.window.showErrorMessage(
                "tsn binary not found. " +
                "Build with `cargo build --release --bin tsn` or set tsn.cli.path."
            );
            return undefined;
        }
        return new vscode.DebugAdapterInlineImplementation(
            new TsnDebugAdapter(cliPath, session.configuration)
        );
    }
}

/**
 * Minimal inline Debug Adapter Protocol implementation.
 * Supports launch-only (run to completion; no step debugging).
 *
 * DAP request flow:
 *   initialize  → capabilities + initialized event
 *   launch      → spawn tsn, stream stdout/stderr as output events
 *   threads     → single "main" thread
 *   disconnect  → kill process
 */
class TsnDebugAdapter {
    /**
     * @param {string} cliPath
     * @param {Record<string, unknown>} launchConfig
     */
    constructor(cliPath, launchConfig) {
        this._cliPath = cliPath;
        this._launchConfig = launchConfig;
        this._seq = 1;
        this._proc = null;
        this._emitter = new vscode.EventEmitter();
        /** @type {vscode.Event<import("vscode").DebugProtocolMessage>} */
        this.onDidSendMessage = this._emitter.event;
    }

    /** @param {import("vscode").DebugProtocolMessage} msg */
    handleMessage(msg) {
        switch (msg.command) {
            case "initialize":
                this._respond(msg, {
                    supportsConfigurationDoneRequest: true,
                    supportsTerminateRequest:         true,
                });
                this._event("initialized");
                break;

            case "launch":
                this._respond(msg);
                this._launch(msg.arguments ?? this._launchConfig);
                break;

            case "configurationDone":
                this._respond(msg);
                break;

            case "threads":
                this._respond(msg, { threads: [{ id: 1, name: "main" }] });
                break;

            case "stackTrace":
                this._respond(msg, { stackFrames: [], totalFrames: 0 });
                break;

            case "scopes":
                this._respond(msg, { scopes: [] });
                break;

            case "variables":
                this._respond(msg, { variables: [] });
                break;

            case "setBreakpoints":
                // Acknowledge without enabling; TSN has no step debugger yet.
                this._respond(msg, {
                    breakpoints: (msg.arguments?.breakpoints ?? []).map(() => ({ verified: false }))
                });
                break;

            case "setFunctionBreakpoints":
                this._respond(msg, { breakpoints: [] });
                break;

            case "setExceptionBreakpoints":
                this._respond(msg, { filters: [] });
                break;

            case "terminate":
            case "disconnect":
                if (this._proc) { this._proc.kill(); this._proc = null; }
                this._respond(msg);
                break;

            default:
                // Unknown requests: respond with failure so the IDE doesn't hang.
                this._emitter.fire({
                    type:        "response",
                    seq:         this._seq++,
                    request_seq: msg.seq,
                    command:     msg.command,
                    success:     false,
                    message:     `unsupported request: ${msg.command}`,
                });
        }
    }

    /** @param {Record<string, unknown>} cfg */
    _launch(cfg) {
        const file = /** @type {string} */ (cfg.program);
        if (!file) {
            this._output("stderr", "tsn: 'program' not set in launch configuration.\n");
            this._event("terminated");
            return;
        }

        const phases  = /** @type {string[]} */ (cfg.debugPhases ?? []);
        const verbose = /** @type {boolean}  */ (cfg.verbose ?? false);
        const noRun   = /** @type {boolean}  */ (cfg.noRun   ?? false);
        const extra   = /** @type {string[]} */ (cfg.args    ?? []);

        const args = [file];
        if (verbose) args.push("--verbose");
        if (noRun)   args.push("--noRun");
        if (phases.length > 0) args.push(`--debug=${phases.join(",")}`);
        if (extra.length  > 0) args.push("--", ...extra);

        const cwd = /** @type {string | undefined} */ (cfg.cwd) ?? path.dirname(file);

        this._event("process", {
            name:           path.basename(file),
            isLocalProcess: true,
            startMethod:    "launch",
        });

        const proc = spawn(this._cliPath, args, { cwd });
        this._proc = proc;

        proc.stdout.on("data", (d) => this._output("stdout", d.toString()));
        proc.stderr.on("data", (d) => this._output("stderr", d.toString()));

        proc.on("error", (err) => {
            this._output("stderr", `Error spawning tsn: ${err.message}\n`);
            this._event("exited",     { exitCode: 1 });
            this._event("terminated", {});
            this._proc = null;
        });

        proc.on("close", (code) => {
            this._output("console", `\nProcess exited with code ${code ?? 0}\n`);
            this._event("exited",     { exitCode: code ?? 0 });
            this._event("terminated", {});
            this._proc = null;
        });
    }

    // ── DAP helpers ──────────────────────────────────────────────────────────

    /**
     * @param {Record<string, unknown>} req
     * @param {Record<string, unknown>} [body]
     */
    _respond(req, body = {}) {
        this._emitter.fire({
            type:        "response",
            seq:         this._seq++,
            request_seq: req.seq,
            command:     req.command,
            success:     true,
            body,
        });
    }

    /**
     * @param {string} event
     * @param {Record<string, unknown>} [body]
     */
    _event(event, body = {}) {
        this._emitter.fire({
            type:  "event",
            seq:   this._seq++,
            event,
            body,
        });
    }

    /**
     * @param {"stdout"|"stderr"|"console"} category
     * @param {string} text
     */
    _output(category, text) {
        this._event("output", { category, output: text });
    }

    dispose() {
        if (this._proc) { this._proc.kill(); this._proc = null; }
        this._emitter.dispose();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Debug configuration provider
// ─────────────────────────────────────────────────────────────────────────────

class TsnDebugConfigProvider {
    /**
     * Called when the user opens "Add Configuration" for a TSN project.
     * Returns the default launch configurations shown in the picker.
     * @returns {vscode.DebugConfiguration[]}
     */
    provideDebugConfigurations() {
        return [
            {
                type:        "tsn",
                request:     "launch",
                name:        "Run TSN File",
                program:     "${file}",
                args:         [],
                cwd:         "${workspaceFolder}",
                verbose:     false,
                debugPhases: [],
            },
        ];
    }

    /**
     * Called whenever a debug session is about to start.
     * If no launch.json config exists (empty config), auto-fill from the active editor.
     * @param {vscode.WorkspaceFolder | undefined} _folder
     * @param {vscode.DebugConfiguration} config
     * @returns {vscode.DebugConfiguration | null}
     */
    resolveDebugConfiguration(_folder, config) {
        // No launch.json present: auto-generate from active editor.
        if (!config.type && !config.request && !config.name) {
            const editor = vscode.window.activeTextEditor;
            if (editor && editor.document.languageId === "tsn") {
                return {
                    type:        "tsn",
                    request:     "launch",
                    name:        "Run TSN File",
                    program:     editor.document.fileName,
                    args:         [],
                    cwd:         path.dirname(editor.document.fileName),
                    verbose:     false,
                    debugPhases: [],
                };
            }
            return null; // Nothing to debug.
        }

        // Ensure 'program' is set.
        if (!config.program) {
            vscode.window.showErrorMessage(
                "TSN debug: 'program' is required in the launch configuration."
            );
            return null;
        }

        return config;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LSP client helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Create (but do not start) an LSP client pointing at the given binary.
 * @param {string} binaryPath
 * @param {vscode.OutputChannel} outputChannel
 * @returns {LanguageClient}
 */
function createClient(binaryPath, outputChannel) {
    const serverOptions = {
        command:   binaryPath,
        transport: TransportKind.stdio,
    };
    
    // Explicitly disable the default output channel if one wasn't provided,
    // although in our current implementation we always provide one.
    const clientOptions = {
        documentSelector: [{ scheme: "file", language: "tsn" }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher("**/*.tsn"),
        },
        outputChannel: outputChannel,
    };

    return new LanguageClient("tsn", "TSN Language Server", serverOptions, clientOptions);
}

async function stopClient() {
    if (client) {
        await client.stop().catch(() => {});
        await client.dispose().catch(() => {});
        client = undefined;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Binary resolution
// ─────────────────────────────────────────────────────────────────────────────

const BIN_EXT = process.platform === "win32" ? ".exe" : "";

/**
 * Resolve the tsn binary.
 * Priority: tsn.cli.path setting → TSN_HOME/bin → target/release → target/debug → PATH
 * @param {vscode.ExtensionContext} context
 * @returns {string | undefined}
 */
function resolveCliPath(context) {
    const override = vscode.workspace.getConfiguration("tsn").get("cli.path");
    if (override && fs.existsSync(override)) return override;

    const root = path.resolve(context.extensionPath, "..");
    const candidates = [
        tsnHomeBinPath(`tsn${BIN_EXT}`),
        path.join(root, "target", "release", `tsn${BIN_EXT}`),
        path.join(root, "target", "debug",   `tsn${BIN_EXT}`),
        // Backward compatibility with previous naming.
        path.join(root, "target", "release", `tsn-cli${BIN_EXT}`),
        path.join(root, "target", "debug",   `tsn-cli${BIN_EXT}`),
    ].filter(Boolean);

    const found = candidates.find((p) => fs.existsSync(p));
    if (found) return found;
    return findOnPath(`tsn${BIN_EXT}`) ?? findOnPath("tsn");
}

function registerTsnCodeLensProvider() {
    const selector = { language: "tsn", scheme: "file" };
    return vscode.languages.registerCodeLensProvider(selector, {
        provideCodeLenses(document) {
            const cfg = vscode.workspace.getConfiguration("tsn");
            const enabled = cfg.get("codeLens.enabled", true);
            if (!enabled) return [];

            const firstLine = new vscode.Range(0, 0, 0, 0);
            return [
                new vscode.CodeLens(firstLine, {
                    title: "Run",
                    command: "tsn.runFile",
                    arguments: [document.uri],
                }),
                new vscode.CodeLens(firstLine, {
                    title: "Bench",
                    command: "tsn.benchFile",
                    arguments: [document.uri],
                }),
                new vscode.CodeLens(firstLine, {
                    title: "Disasm",
                    command: "tsn.disasmFile",
                    arguments: [document.uri],
                }),
                new vscode.CodeLens(firstLine, {
                    title: "AST",
                    command: "tsn.showAst",
                    arguments: [document.uri],
                }),
            ];
        },
    });
}

/**
 * Resolve the tsn-lsp binary.
 * Priority: tsn.server.path setting → TSN_HOME/bin → target/release → target/debug → PATH
 * @param {vscode.ExtensionContext} context
 * @returns {string | undefined}
 */
function resolveLspPath(context) {
    const override = vscode.workspace.getConfiguration("tsn").get("server.path");
    if (override && fs.existsSync(override)) return override;

    const root = path.resolve(context.extensionPath, "..");
    const candidates = [
        tsnHomeBinPath(`tsn-lsp${BIN_EXT}`),
        path.join(root, "target", "release", `tsn-lsp${BIN_EXT}`),
        path.join(root, "target", "debug",   `tsn-lsp${BIN_EXT}`),
    ].filter(Boolean);

    const found = candidates.find((p) => fs.existsSync(p));
    if (found) return found;
    return findOnPath(`tsn-lsp${BIN_EXT}`) ?? findOnPath("tsn-lsp");
}

/**
 * @param {string} binFile
 * @returns {string | undefined}
 */
function tsnHomeBinPath(binFile) {
    const home = process.env.TSN_HOME;
    if (!home) return undefined;
    return path.join(home, "bin", binFile);
}

/**
 * @param {string} executable
 * @returns {string | undefined}
 */
function findOnPath(executable) {
    const pathValue = process.env.PATH;
    if (!pathValue) return undefined;
    const dirs = pathValue.split(path.delimiter);
    for (const dir of dirs) {
        if (!dir) continue;
        const full = path.join(dir, executable);
        if (fs.existsSync(full)) return full;
    }
    return undefined;
}

