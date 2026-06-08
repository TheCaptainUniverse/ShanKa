import { ref } from "vue";
import type { HudUpdate } from "@shared";

const currentHud = ref<HudUpdate>({ status: "idle" });

export function useHud() {
  function setHud(update: HudUpdate) {
    currentHud.value = update;
  }

  return {
    currentHud,
    setHud,
  };
}
