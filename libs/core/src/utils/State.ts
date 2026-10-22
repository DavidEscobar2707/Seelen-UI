import { invoke as tauriInvoke } from "@tauri-apps/api/core";

import type {
  AllSeelenCommandArguments,
  AllSeelenCommandReturns,
  AllSeelenEventPayloads,
  SeelenCommand,
  SeelenEvent,
  UnSubscriber,
} from "../handlers/mod.ts";
import { subscribe } from "../handlers/mod.ts";

// deno-lint-ignore no-explicit-any
interface ConstructorWithSingleArg<T = any> {
  // deno-lint-ignore no-explicit-any
  new (arg0: T): any;
}

export async function newFromInvoke<
  Command extends SeelenCommand,
  This extends ConstructorWithSingleArg<AllSeelenCommandReturns[Command]>,
>(
  Class: This,
  command: Command,
  args?: NonNullable<AllSeelenCommandArguments[Command]>,
): Promise<InstanceType<This>> {
  return new Class(await tauriInvoke(command, args));
}

export function newOnEvent<
  Event extends SeelenEvent,
  This extends ConstructorWithSingleArg<AllSeelenEventPayloads[Event]>,
>(
  cb: (instance: InstanceType<This>) => void,
  Class: This,
  event: Event,
): Promise<UnSubscriber> {
  return subscribe(event, (eventData) => {
    cb(new Class(eventData.payload as AllSeelenEventPayloads[Event]));
  });
}
