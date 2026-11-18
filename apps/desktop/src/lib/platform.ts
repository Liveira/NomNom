/** Detects whether the UI is running on Linux or Windows so labels match the desktop. */
export const isLinux: boolean =
  typeof navigator !== 'undefined' && /linux/i.test(navigator.userAgent) && !/android/i.test(navigator.userAgent);

export const trashName: string = isLinux ? 'Trash' : 'Recycle Bin';
export const trashPhrase: string = isLinux ? 'Trash' : 'the Recycle Bin';
export const fileManagerName: string = isLinux ? 'file manager' : 'Explorer';
export const adminName: string = isLinux ? 'root' : 'Administrator';
export const nativeHostExeName: string = isLinux ? 'nomnom-native-host' : 'nomnom-native-host.exe';
