import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'reminders',
  name: 'Reminders',
  view: () => import('./View.svelte'),
});
