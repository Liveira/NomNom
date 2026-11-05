import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'organizer',
  name: 'File Organizer',
  view: () => import('./View.svelte'),
});
