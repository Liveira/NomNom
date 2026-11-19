import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'toolbox',
  name: 'Text toolbox',
  view: () => import('./View.svelte'),
});
