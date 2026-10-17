/** A shared, slowly ticking "now" so relative times and the clock stay current. */
export const now = $state({ value: Date.now() });

let timer: ReturnType<typeof setInterval> | undefined;

export function startClock() {
  if (timer) return;
  // Align ticks with the start of each minute so the clock flips on time.
  const tick = () => (now.value = Date.now());
  const delay = 60000 - (Date.now() % 60000);
  setTimeout(() => {
    tick();
    timer = setInterval(tick, 60000);
  }, delay);
  // Relative times refresh more often than the minute boundary.
  setInterval(tick, 15000);
}
