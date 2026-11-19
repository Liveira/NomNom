import { reminderIpc } from '../../lib/ipc';

/** Kept outside the page, so the stopwatch and the timer go on while another page is open. */
class Clocks {
  // Stopwatch
  swRunning = $state(false);
  private swStart = 0;
  private swBefore = 0;
  swMs = $state(0);
  laps = $state<number[]>([]);
  private raf = 0;

  swToggle() {
    if (this.swRunning) {
      this.swBefore += performance.now() - this.swStart;
      this.swRunning = false;
      cancelAnimationFrame(this.raf);
      this.swMs = this.swBefore;
    } else {
      this.swStart = performance.now();
      this.swRunning = true;
      const loop = () => {
        this.swMs = this.swBefore + performance.now() - this.swStart;
        if (this.swRunning) this.raf = requestAnimationFrame(loop);
      };
      loop();
    }
  }
  swLap() {
    if (this.swRunning) this.laps = [this.swMs, ...this.laps];
  }
  swReset() {
    this.swRunning = false;
    cancelAnimationFrame(this.raf);
    this.swBefore = 0;
    this.swMs = 0;
    this.laps = [];
  }

  // Timer: a reminder, so the card shows even when NomNom's window is closed.
  timerDue = $state<number | null>(null);
  timerLabel = $state('');
  nowSec = $state(Math.floor(Date.now() / 1000));
  private ticker: ReturnType<typeof setInterval> | undefined;

  get timerLeft(): number {
    return this.timerDue === null ? 0 : Math.max(0, this.timerDue - this.nowSec);
  }

  async timerStart(seconds: number, label: string) {
    const due = Math.floor(Date.now() / 1000) + seconds;
    await reminderIpc.addAt(label, due, null);
    this.timerDue = due;
    this.timerLabel = label;
    this.nowSec = Math.floor(Date.now() / 1000);
    clearInterval(this.ticker);
    this.ticker = setInterval(() => {
      this.nowSec = Math.floor(Date.now() / 1000);
      if (this.timerDue !== null && this.nowSec > this.timerDue + 2) this.timerClear();
    }, 500);
  }

  async timerCancel() {
    if (this.timerDue === null) return;
    const due = this.timerDue;
    const text = this.timerLabel;
    for (const r of await reminderIpc.list()) {
      if (!r.done && r.text === text && r.due_at === due) await reminderIpc.remove(r.id);
    }
    this.timerClear();
  }

  private timerClear() {
    clearInterval(this.ticker);
    this.timerDue = null;
    this.timerLabel = '';
  }
}

export const clocks = new Clocks();
