import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'rules',
  name: 'Folder rules',
  view: () => import('./View.svelte'),
});
