/** "12,340 of 500,000 characters used this month." */
export function usageLine(used: number, limit: number): string {
  const fmt = (n: number) => n.toLocaleString('en-US');
  return `${fmt(used)} of ${fmt(limit)} characters used this month.`;
}

/** The language DeepL recognised, as a short name for the small window ("PT" becomes "PT"). */
export function fromLabel(detected: string): string {
  return detected ? detected.toUpperCase() : '';
}
