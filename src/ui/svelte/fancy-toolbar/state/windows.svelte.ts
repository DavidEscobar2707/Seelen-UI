import { Widget } from "@seelen-ui/lib";
import { settingsState } from "./settings.svelte.ts";
import { focused, interactables, widgetStatuses } from "./getters.svelte.ts";

export { focused, interactables, widgetStatuses };

const widget = Widget.getCurrent();

const _topInteractableWindow = $derived(
  interactables.value
    .toSorted((a, b) => b.lastForegroundAt - a.lastForegroundAt)
    .find((w) => w.monitor === widget.decoded.monitorId && !w.isIconic),
);

const _thereIsMaximizedOnBg = $derived(
  interactables.value.some(
    (w) => !w.isIconic && w.isZoomed && w.monitor === widget.decoded.monitorId,
  ),
);

const _isTbOverlapped = $derived.by(() => {
  // If foreground is not in interactable windows, return false directly, this handled start menu or desktop focus cases.
  const foreground = focused.value;
  if (!interactables.value.some((w) => w.hwnd === foreground.hwnd)) {
    return false;
  }

  // Check if any interactable window overlaps with the hitbox
  const a = settingsState.widgetRect;
  for (const app of interactables.value) {
    if (app.monitor !== widget.decoded.monitorId || app.isIconic || !app.rect) continue;
    const b = app.rect;

    if (!(a.right <= b.left || a.left >= b.right || a.bottom <= b.top || a.top >= b.bottom)) {
      return true;
    }
  }

  return false;
});

class WindowsState {
  get topInteractableWindow() {
    return _topInteractableWindow;
  }

  get thereIsMaximizedOnBg() {
    return _thereIsMaximizedOnBg;
  }

  get isTbOverlapped() {
    return _isTbOverlapped;
  }
}

export const windowsState = new WindowsState();
