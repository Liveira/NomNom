import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'text',
  name: 'Translator',
  view: () => import('./View.svelte'),
});
