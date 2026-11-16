/** The faders the "Auswählen" keys picked; "Lauter / Leiser" keys with "Auswahl verwenden" move them. */
const picked = new Set<string>();
const listeners: (() => void)[] = [];
const changed = (): void => listeners.forEach((f) => f());

export const selection = {
  has: (id: string): boolean => picked.has(id),
  toggle(id: string): void {
    if (!picked.delete(id)) picked.add(id);
    changed();
  },
  clear(): void {
    picked.clear();
    changed();
  },
  onChange: (f: () => void): number => listeners.push(f),
  get size(): number {
    return picked.size;
  },
  /** Strip inputs and master outputs, in the form of the settings of a group key. */
  split(): { strips: number[]; masters: number[] } {
    const strips: number[] = [];
    const masters: number[] = [];
    for (const id of picked) {
      const n = Number(id.slice(1));
      if (id[0] === "s") strips.push(n);
      else if (id[0] === "m") masters.push(n);
    }
    return { strips, masters };
  },
};
