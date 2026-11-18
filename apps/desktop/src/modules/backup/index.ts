import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'backup',
  name: 'Backup',
  view: () => import('./View.svelte'),
});
