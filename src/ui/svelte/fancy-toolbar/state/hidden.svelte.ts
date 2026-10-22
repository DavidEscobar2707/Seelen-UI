import { SeelenEvent, subscribe } from "@seelen-ui/lib";
import { HideMode } from "@seelen-ui/lib/types";
import { isThisWebviewFocused, isTouchPrimary } from "libs/ui/svelte/utils/signals.svelte.ts";
import { mousePos, virtualDesktops } from "./getters.svelte.ts";
import { settingsState } from "./settings.svelte.ts";
import { systemState } from "./system.svelte.ts";
import { windowsState } from "./windows.svelte.ts";

const isSwitchingWorkspace = $derived(virtualDesktops.value.switching);

let _hiddenByAutohide = $state(false);
let _isDraggingItem = $state(false);

export const hiddenByAutohide = {
  get value() {
    return _hiddenByAutohide;
  },
};

export function setToolbarIsDraggingItem(isDragging: boolean): void {
  _isDraggingItem = isDragging;
}

// Only the auto-hide modes need the mouse position. Subscribing wakes the 10 Hz global mouse hook, so default
// (HideMode.Never) toolbars do not subscribe at all.
$effect.root(() => {
  $effect(() => {
    if (settingsState.hideMode === HideMode.Never) return;
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    subscribe(SeelenEvent.GlobalMouseMove, mousePos.setByPayload).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  });
});

$effect.root(() => {
  let timeout: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    if (isSwitchingWorkspace) {
      if (timeout) {
        clearTimeout(timeout);
        timeout = null;
      }
      return;
    }

    const { delayToHide, delayToShow, hideMode, position } = settingsState;
    const isMouseOverEdge = systemState.mouseAtEdge === position;

    let hidden = false;
    let flush = false;

    switch (hideMode) {
      case HideMode.Never:
        hidden = false;
        flush = true;
        break;
      case HideMode.Always:
        hidden = !isTouchPrimary.value && !isThisWebviewFocused.value && !isMouseOverEdge;
        flush = isTouchPrimary.value;
        break;
      case HideMode.OnOverlap:
        hidden = !isTouchPrimary.value &&
          windowsState.isTbOverlapped &&
          !isThisWebviewFocused.value &&
          !isMouseOverEdge;
        flush = isTouchPrimary.value;
        break;
    }

    if (_isDraggingItem) {
      hidden = false;
      flush = true;
    }

    if (hidden) {
      timeout = setTimeout(() => {
        _hiddenByAutohide = true;
      }, delayToHide);
    } else {
      if (flush) {
        _hiddenByAutohide = false;
      } else {
        timeout = setTimeout(() => {
          _hiddenByAutohide = false;
        }, delayToShow);
      }
    }

    return () => {
      if (timeout) {
        clearTimeout(timeout);
        timeout = null;
      }
    };
  });
});
