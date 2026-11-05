import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'filetools',
  name: 'File Tools',
  view: () => import('./View.svelte'),
});
