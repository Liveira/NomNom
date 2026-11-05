import { send } from './shared/messaging';

const status = document.getElementById('status') as HTMLElement;
const button = document.getElementById('test') as HTMLButtonElement;

async function check() {
  button.disabled = true;
  status.dataset.ok = '';
  status.textContent = 'Checking…';
  const r = await send({ kind: 'ping' });
  status.dataset.ok = String(r.ok);
  status.textContent = r.ok && r.version ? `NomNom is connected (version ${r.version}).` : r.message;
  button.disabled = false;
}

button.addEventListener('click', () => void check());
void check();
