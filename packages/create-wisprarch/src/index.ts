#!/usr/bin/env node

import * as p from "@clack/prompts";
import pc from "picocolors";
import { execSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync, chmodSync } from "node:fs";
import { homedir, platform, arch } from "node:os";
import { join } from "node:path";
import { createWriteStream } from "node:fs";
import { pipeline } from "node:stream/promises";

const LOGO = `
${pc.cyan("░█░█░▀█▀░█▀▀░█▀█░█▀▄░█▀█░█▀▄░█▀▀░█░█")}
${pc.cyan("░█▄█░░█░░▀▀█░█▀▀░█▀▄░█▀█░█▀▄░█░░░█▀█")}
${pc.cyan("░▀░▀░▀▀▀░▀▀▀░▀░░░▀░▀░▀░▀░▀░▀░▀▀▀░▀░▀")}
`;

const PROVIDERS = [
	{
		value: "groq",
		label: "Groq Cloud",
		hint: "Blazing fast (216x realtime) - $0.04/hr",
		requiresKey: true,
		keyName: "GROQ_API_KEY",
		keyUrl: "https://console.groq.com/keys",
	},
	{
		value: "openai-api",
		label: "OpenAI Whisper API",
		hint: "High quality - $0.36/hr",
		requiresKey: true,
		keyName: "OPENAI_API_KEY",
		keyUrl: "https://platform.openai.com/api-keys",
	},
	{
		value: "parakeet-v3",
		label: "Parakeet v3 (Local)",
		hint: "Multilingual, offline, free - requires NVIDIA GPU",
		requiresKey: false,
	},
	{
		value: "parakeet-v2",
		label: "Parakeet v2 (Local)",
		hint: "English only, offline, free - requires NVIDIA GPU",
		requiresKey: false,
	},
	{
		value: "whisper-cpp",
		label: "Whisper.cpp (Local)",
		hint: "Offline, free - CPU/GPU",
		requiresKey: false,
	},
] as const;

const KEYBIND_PRESETS = [
	{ value: "super-r", label: "Super + R", keys: { mod: "SUPER", key: "R" } },
	{ value: "super-shift-r", label: "Super + Shift + R", keys: { mod: "SUPER SHIFT", key: "R" } },
	{ value: "super-v", label: "Super + V", keys: { mod: "SUPER", key: "V" } },
	{ value: "ctrl-shift-space", label: "Ctrl + Shift + Space", keys: { mod: "CTRL SHIFT", key: "SPACE" } },
	{ value: "custom", label: "Custom keybind...", keys: null },
] as const;

interface Config {
	provider: string;
	apiKey?: string;
	model: string;
	language: string;
	keybind: { mod: string; key: string };
	autoPaste: boolean;
	audioFeedback: boolean;
	waybarIntegration: boolean;
	deleteAudioFiles: boolean;
}

async function main() {
	console.clear();
	console.log(LOGO);

	p.intro(pc.bgCyan(pc.black(" WisprArch Installer ")));

	const osType = platform();
	const archType = arch();

	if (osType !== "linux") {
		p.log.warn(`WisprArch is designed for Arch Linux. Detected: ${osType}`);
		const proceed = await p.confirm({
			message: "Continue anyway?",
			initialValue: false,
		});
		if (p.isCancel(proceed) || !proceed) {
			p.cancel("Installation cancelled.");
			process.exit(0);
		}
	}

	const s = p.spinner();

	s.start("Checking system requirements...");
	const requirements = await checkRequirements();
	s.stop("System check complete");

	if (requirements.missing.length > 0) {
		p.log.warn("Missing recommended tools:");
		for (const tool of requirements.missing) {
			console.log(pc.yellow(`  ! ${tool.name}: ${tool.message}`));
		}
		console.log();
	}

	if (requirements.found.length > 0) {
		p.log.success("Found tools:");
		for (const tool of requirements.found) {
			console.log(pc.green(`  ✓ ${tool}`));
		}
		console.log();
	}

	const config = await runPrompts();

	if (p.isCancel(config)) {
		p.cancel("Installation cancelled.");
		process.exit(0);
	}

	s.start("Installing WisprArch...");
	await installBinary(s);
	s.stop("Binary installed");

	s.start("Creating configuration...");
	await createConfig(config);
	s.stop("Configuration created");

	s.start("Setting up systemd service...");
	await setupService();
	s.stop("Service configured");

	if (config.waybarIntegration) {
		p.log.info("Waybar integration enabled. Add this to your waybar config:");
		console.log(pc.dim(`
"custom/wisprarch": {
    "exec": "curl -s http://127.0.0.1:3737/waybar",
    "return-type": "json",
    "interval": 1,
    "on-click": "curl -X POST http://127.0.0.1:3737/toggle"
}`));
	}

	p.note(
		`${pc.cyan("Start service:")} systemctl --user start wisprarch
${pc.cyan("Enable on boot:")} systemctl --user enable wisprarch
${pc.cyan("Test keybind:")} Press ${pc.bold(config.keybind.mod + " + " + config.keybind.key)} to record

${pc.dim("Config file:")} ~/.config/wisprarch/config.toml
${pc.dim("Logs:")} journalctl --user -u wisprarch -f`,
		"Next steps"
	);

	p.outro(pc.green("✨ WisprArch installed successfully!"));
}

async function runPrompts(): Promise<Config> {
	const provider = await p.select({
		message: "Choose your transcription provider:",
		options: PROVIDERS.map((p) => ({
			value: p.value,
			label: p.label,
			hint: p.hint,
		})),
	});

	if (p.isCancel(provider)) {
		p.cancel("Installation cancelled.");
		process.exit(0);
	}

	const selectedProvider = PROVIDERS.find((p) => p.value === provider)!;
	let apiKey: string | undefined;

	if (selectedProvider.requiresKey) {
		p.log.info(`Get your API key from: ${pc.cyan(selectedProvider.keyUrl)}`);

		const key = await p.password({
			message: `Enter your ${selectedProvider.keyName}:`,
			validate: (value) => {
				if (!value || value.length < 10) {
					return "Please enter a valid API key";
				}
			},
		});

		if (p.isCancel(key)) {
			p.cancel("Installation cancelled.");
			process.exit(0);
		}

		apiKey = key;
	}

	const language = await p.select({
		message: "Select transcription language:",
		initialValue: "en",
		options: [
			{ value: "en", label: "English" },
			{ value: "es", label: "Spanish" },
			{ value: "fr", label: "French" },
			{ value: "de", label: "German" },
			{ value: "it", label: "Italian" },
			{ value: "pt", label: "Portuguese" },
			{ value: "zh", label: "Chinese" },
			{ value: "ja", label: "Japanese" },
			{ value: "ko", label: "Korean" },
			{ value: "auto", label: "Auto-detect", hint: "Let the model detect the language" },
		],
	});

	if (p.isCancel(language)) {
		p.cancel("Installation cancelled.");
		process.exit(0);
	}

	p.log.step("Configure keyboard shortcut");

	const keybindChoice = await p.select({
		message: "Choose a keybind to toggle recording:",
		options: KEYBIND_PRESETS.map((k) => ({
			value: k.value,
			label: k.label,
		})),
	});

	if (p.isCancel(keybindChoice)) {
		p.cancel("Installation cancelled.");
		process.exit(0);
	}

	let keybind: { mod: string; key: string };

	if (keybindChoice === "custom") {
		const customMod = await p.text({
			message: "Enter modifier keys (e.g., SUPER, CTRL SHIFT):",
			placeholder: "SUPER SHIFT",
			validate: (value) => {
				if (!value) return "Modifier is required";
			},
		});

		if (p.isCancel(customMod)) {
			p.cancel("Installation cancelled.");
			process.exit(0);
		}

		const customKey = await p.text({
			message: "Enter the main key (e.g., R, V, SPACE):",
			placeholder: "R",
			validate: (value) => {
				if (!value) return "Key is required";
			},
		});

		if (p.isCancel(customKey)) {
			p.cancel("Installation cancelled.");
			process.exit(0);
		}

		keybind = { mod: customMod.toUpperCase(), key: customKey.toUpperCase() };
	} else {
		keybind = KEYBIND_PRESETS.find((k) => k.value === keybindChoice)!.keys!;
	}

	p.log.step("Configure behavior");

	const behavior = await p.group(
		{
			autoPaste: () =>
				p.confirm({
					message: "Auto-paste transcribed text at cursor?",
					initialValue: true,
				}),
			audioFeedback: () =>
				p.confirm({
					message: "Play sound when recording starts/stops?",
					initialValue: true,
				}),
			waybarIntegration: () =>
				p.confirm({
					message: "Enable Waybar status indicator?",
					initialValue: true,
				}),
			deleteAudioFiles: () =>
				p.confirm({
					message: "Delete temporary audio files after transcription?",
					initialValue: true,
				}),
		},
		{
			onCancel: () => {
				p.cancel("Installation cancelled.");
				process.exit(0);
			},
		}
	);

	const model = getModelForProvider(provider as string);

	return {
		provider: provider as string,
		apiKey,
		model,
		language: language as string,
		keybind,
		autoPaste: behavior.autoPaste,
		audioFeedback: behavior.audioFeedback,
		waybarIntegration: behavior.waybarIntegration,
		deleteAudioFiles: behavior.deleteAudioFiles,
	};
}

function getModelForProvider(provider: string): string {
	switch (provider) {
		case "groq":
		case "openai-api":
			return "whisper-large-v3-turbo";
		case "parakeet-v3":
			return "parakeet-v3";
		case "parakeet-v2":
			return "parakeet-v2";
		case "whisper-cpp":
			return "ggml-large-v3-turbo-q5_1";
		default:
			return "whisper-large-v3-turbo";
	}
}

interface RequirementResult {
	found: string[];
	missing: { name: string; message: string }[];
}

async function checkRequirements(): Promise<RequirementResult> {
	const found: string[] = [];
	const missing: { name: string; message: string }[] = [];

	const tools = [
		{ name: "wtype", message: "Install with: sudo pacman -S wtype (for auto-paste)" },
		{ name: "ydotool", message: "Install with: sudo pacman -S ydotool (alternative for auto-paste)" },
		{ name: "wl-copy", message: "Install with: sudo pacman -S wl-clipboard (for clipboard)" },
		{ name: "curl", message: "Install with: sudo pacman -S curl" },
	];

	for (const tool of tools) {
		if (commandExists(tool.name)) {
			found.push(tool.name);
		} else if (tool.name === "wtype" || tool.name === "ydotool") {
			if (!found.includes("wtype") && !found.includes("ydotool")) {
				missing.push(tool);
			}
		}
	}

	return { found, missing };
}

function commandExists(cmd: string): boolean {
	try {
		execSync(`which ${cmd}`, { stdio: "ignore" });
		return true;
	} catch {
		return false;
	}
}

async function installBinary(spinner: ReturnType<typeof p.spinner>): Promise<void> {
	const installDir = "/usr/local/bin";
	const binaryName = "wisprarch";
	const binaryPath = join(installDir, binaryName);

	const osType = platform();
	const archType = arch();

	let targetId: string;
	if (osType === "linux" && archType === "x64") {
		targetId = "linux-x86_64-gnu";
	} else if (osType === "linux" && archType === "arm64") {
		targetId = "linux-aarch64-gnu";
	} else if (osType === "darwin" && archType === "arm64") {
		targetId = "macos-aarch64";
	} else if (osType === "darwin" && archType === "x64") {
		targetId = "macos-x86_64";
	} else {
		throw new Error(`Unsupported platform: ${osType}-${archType}`);
	}

	spinner.message("Downloading latest release...");

	const releaseUrl = `https://github.com/wisprarch/wisprarch/releases/latest/download/wisprarch-${targetId}.tar.gz`;
	const tempDir = join(homedir(), ".cache", "wisprarch-install");

	mkdirSync(tempDir, { recursive: true });

	const tarPath = join(tempDir, "wisprarch.tar.gz");

	try {
		const response = await fetch(releaseUrl);
		if (!response.ok) {
			throw new Error(`Failed to download: ${response.statusText}`);
		}

		const fileStream = createWriteStream(tarPath);
		await pipeline(response.body as any, fileStream);

		spinner.message("Extracting...");
		execSync(`tar -xzf "${tarPath}" -C "${tempDir}"`, { stdio: "ignore" });

		spinner.message("Installing binary (may require sudo)...");

		const extractedBinary = join(tempDir, "wisprarch");
		if (existsSync(extractedBinary)) {
			try {
				execSync(`sudo cp "${extractedBinary}" "${binaryPath}"`, { stdio: "inherit" });
				execSync(`sudo chmod +x "${binaryPath}"`, { stdio: "ignore" });
			} catch {
				execSync(`cp "${extractedBinary}" "${join(homedir(), ".local", "bin", binaryName)}"`, {
					stdio: "ignore",
				});
				p.log.warn(`Installed to ~/.local/bin (add to PATH if needed)`);
			}
		}

		execSync(`rm -rf "${tempDir}"`, { stdio: "ignore" });
	} catch (error) {
		p.log.warn("Could not download pre-built binary. Building from source...");
		spinner.message("Building from source (this may take a few minutes)...");

		try {
			execSync("cargo build --release", { stdio: "ignore", cwd: process.cwd() });
			const builtBinary = join(process.cwd(), "target", "release", "wisprarch");
			execSync(`sudo cp "${builtBinary}" "${binaryPath}"`, { stdio: "inherit" });
			execSync(`sudo chmod +x "${binaryPath}"`, { stdio: "ignore" });
		} catch (buildError) {
			throw new Error("Failed to install binary. Please build manually with: cargo build --release");
		}
	}
}

async function createConfig(config: Config): Promise<void> {
	const configDir = join(homedir(), ".config", "wisprarch");
	const configPath = join(configDir, "config.toml");

	mkdirSync(configDir, { recursive: true });

	const toml = `[whisper]
provider = "${config.provider}"
model = "${config.model}"
language = "${config.language}"${config.apiKey ? `\napi_key = "${config.apiKey}"` : ""}

[behavior]
auto_paste = ${config.autoPaste}
delete_audio_files = ${config.deleteAudioFiles}
audio_feedback = ${config.audioFeedback}

[ui.waybar]
idle_text = "󰓃"
recording_text = "󰻃"
idle_tooltip = "Press ${config.keybind.mod} + ${config.keybind.key} to record"
recording_tooltip = "Recording... Press ${config.keybind.mod} + ${config.keybind.key} to stop"

[wayland]
input_method = "wtype"
`;

	writeFileSync(configPath, toml);

	const hyprConfigDir = join(homedir(), ".config", "hypr");
	if (existsSync(hyprConfigDir)) {
		const keybindLine = `bindd = ${config.keybind.mod}, ${config.keybind.key}, wisprarch, exec, curl -X POST http://127.0.0.1:3737/toggle`;
		p.log.info(`Add this to your hyprland.conf:\n${pc.cyan(keybindLine)}`);
	}
}

async function setupService(): Promise<void> {
	const serviceDir = join(homedir(), ".config", "systemd", "user");
	const servicePath = join(serviceDir, "wisprarch.service");

	mkdirSync(serviceDir, { recursive: true });

	const serviceContent = `[Unit]
Description=WisprArch Speech-to-Text Service
Documentation=https://github.com/wisprarch/wisprarch
After=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/local/bin/wisprarch
Restart=always
RestartSec=5

StandardOutput=journal
StandardError=journal
Environment="RUST_LOG=info"

PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=%h/.config/wisprarch %h/.local/share/wisprarch %t
MemoryMax=6G
CPUQuota=80%

[Install]
WantedBy=default.target
`;

	writeFileSync(servicePath, serviceContent);

	try {
		execSync("systemctl --user daemon-reload", { stdio: "ignore" });
	} catch {
		p.log.warn("Could not reload systemd. Run: systemctl --user daemon-reload");
	}
}

main().catch((error) => {
	p.log.error(error.message);
	process.exit(1);
});
