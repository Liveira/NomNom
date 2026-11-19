import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'clipboard',
  name: 'Clipboard',
  view: () => import('./View.svelte'),
});
