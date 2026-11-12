import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'storage',
  name: 'Disk Analyzer',
  view: () => import('./View.svelte'),
});
