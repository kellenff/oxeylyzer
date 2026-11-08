import { listen } from "@tauri-apps/api/event";
import { cancelGenerate, saveGenerated, startGenerate } from "./api";
import { createGenerationStore } from "./generationStore";

export const generationStore = createGenerationStore({
  listen,
  startGenerate,
  cancelGenerate,
  saveGenerated,
});
