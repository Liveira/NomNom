import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'snippets',
  name: 'Snippets',
  view: () => import('./View.svelte'),
});
