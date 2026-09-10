import { createResource, For, Show, Switch, Match } from "solid-js";
import { useNavigate } from "@solidjs/router";
import { propfind } from "../lib/api";
import { Folder, Loader2 } from "lucide-solid";

export default function SpacesList() {
  const nav = useNavigate();
  const [spaces, { refetch }] = createResource(async () => propfind("", "1", "_spaces"));

  return (
    <div class="p-6 max-w-3xl mx-auto h-full flex flex-col">
      <h1 class="text-xl font-semibold mb-4 shrink-0">Spaces</h1>
      <div class="flex-1 overflow-y-auto min-h-0">
        <Switch>
          <Match when={spaces.loading}>
            <div class="flex justify-center pt-16"><Loader2 size={24} class="animate-spin text-[var(--text-tertiary)]" /></div>
          </Match>
          <Match when={spaces.error}>
            <p class="text-sm text-[var(--danger)]">{String(spaces.error)}</p>
          </Match>
          <Match when={(spaces() ?? []).length === 0}>
            <p class="text-sm text-[var(--text-tertiary)] text-center pt-16">No spaces yet — admins can create one with "New space"</p>
          </Match>
          <Match when={true}>
            <For each={spaces() ?? []}>
              {(s) => (
                <button
                  onClick={() => nav(`/ui/spaces/${s.name}`)}
                  class="w-full text-left group flex items-center gap-3 px-4 py-3.5 glass rounded-xl mb-2 hover:bg-[var(--bg-raised)] transition-colors"
                >
                  <Folder size={20} class="text-[var(--accent)] shrink-0" />
                  <span class="flex-1">
                    <span class="block text-sm font-medium">{s.name}</span>
                  </span>
                </button>
              )}
            </For>
          </Match>
        </Switch>
      </div>
    </div>
  );
}
