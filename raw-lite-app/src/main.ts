import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import "./styles.css";

type ProgressPayload = {
  processed: number;
  total: number;
  filename: string;
  succeeded: boolean;
};

type FailurePayload = {
  filename: string;
  reason: string;
};

type SummaryPayload = {
  succeeded: number;
  failed: FailurePayload[];
  folderOpenError: string | null;
};

const rawExtensions = [
  "3fr", "arw", "cr2", "cr3", "dcr", "dng", "erf", "fff", "iiq", "kdc", "mef",
  "mos", "mrw", "nef", "nrw", "orf", "pef", "raf", "raw", "rw2", "rwl", "sr2",
  "srf", "srw", "x3f",
];

const byId = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const outputPath = byId<HTMLParagraphElement>("output-path");
const chooseOutputButton = byId<HTMLButtonElement>("choose-output");
const chooseFilesButton = byId<HTMLButtonElement>("choose-files");
const dropZone = byId<HTMLElement>("drop-zone");
const progressPanel = byId<HTMLElement>("progress-panel");
const progressBar = byId<HTMLProgressElement>("progress");
const progressCount = byId<HTMLOutputElement>("progress-count");
const currentFile = byId<HTMLParagraphElement>("current-file");
const summaryPanel = byId<HTMLElement>("summary");
const summaryCopy = byId<HTMLParagraphElement>("summary-copy");
const folderWarning = byId<HTMLParagraphElement>("folder-warning");
const failures = byId<HTMLUListElement>("failures");
const errorMessage = byId<HTMLParagraphElement>("error");
const maxDimension = byId<HTMLInputElement>("max-dimension");
const quality = byId<HTMLInputElement>("quality");
const qualityValue = byId<HTMLOutputElement>("quality-value");

let selectedOutput: string | null = null;
let converting = false;

function setBusy(busy: boolean) {
  converting = busy;
  chooseOutputButton.disabled = busy;
  chooseFilesButton.disabled = busy;
  maxDimension.disabled = busy;
  quality.disabled = busy;
  dropZone.classList.toggle("is-disabled", busy);
}

async function chooseOutput() {
  if (converting) return;
  const selected = await open({ directory: true, multiple: false, title: "Choose output folder" });
  if (typeof selected === "string") {
    selectedOutput = selected;
    outputPath.textContent = selected;
    outputPath.title = selected;
    dropZone.classList.add("is-ready");
  }
}

function resetResults() {
  errorMessage.hidden = true;
  summaryPanel.hidden = true;
  folderWarning.hidden = true;
  failures.replaceChildren();
}

async function convert(paths: string[]) {
  if (converting || paths.length === 0) return;
  if (!selectedOutput) {
    await chooseOutput();
    if (!selectedOutput) {
      errorMessage.textContent = "Choose an output folder before adding RAW files.";
      errorMessage.hidden = false;
      return;
    }
  }

  resetResults();
  setBusy(true);
  progressPanel.hidden = false;
  progressBar.value = 0;
  progressBar.max = 1;
  progressCount.value = "Preparing…";
  currentFile.textContent = "Finding supported RAW files…";

  try {
    const result = await invoke<SummaryPayload>("start_conversion", {
      paths,
      outputDirectory: selectedOutput,
      maxDimension: Number(maxDimension.value),
      quality: Number(quality.value),
    });
    summaryCopy.textContent = `${result.succeeded} ${result.succeeded === 1 ? "photo" : "photos"} converted.`;
    for (const failure of result.failed) {
      const item = document.createElement("li");
      const name = document.createElement("strong");
      name.textContent = failure.filename;
      item.append(name, ` — ${failure.reason}`);
      failures.append(item);
    }
    if (result.folderOpenError) {
      folderWarning.textContent = "The output folder could not be opened automatically. Use the path shown above.";
      folderWarning.hidden = false;
    }
    summaryPanel.hidden = false;
  } catch (error) {
    errorMessage.textContent = String(error);
    errorMessage.hidden = false;
  } finally {
    setBusy(false);
  }
}

await listen<ProgressPayload>("conversion-progress", ({ payload }) => {
  progressBar.max = payload.total;
  progressBar.value = payload.processed;
  progressCount.value = `${payload.processed} / ${payload.total}`;
  currentFile.textContent = `${payload.succeeded ? "Converted" : "Skipped"} ${payload.filename}`;
});

await getCurrentWebview().onDragDropEvent(({ payload }) => {
  if (payload.type === "over" || payload.type === "enter") {
    dropZone.classList.add("is-dragging");
  } else {
    dropZone.classList.remove("is-dragging");
  }
  if (payload.type === "drop") void convert(payload.paths);
});

chooseOutputButton.addEventListener("click", () => void chooseOutput());
chooseFilesButton.addEventListener("click", async () => {
  const selected = await open({
    multiple: true,
    directory: false,
    title: "Choose RAW files",
    filters: [{ name: "RAW photos", extensions: rawExtensions }],
  });
  if (selected) void convert(Array.isArray(selected) ? selected : [selected]);
});
quality.addEventListener("input", () => {
  qualityValue.value = quality.value;
});

void chooseOutput();
