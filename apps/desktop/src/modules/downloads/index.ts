import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'downloads',
  name: 'NomNom Downloader',
  view: () => import('./View.svelte'),
});
