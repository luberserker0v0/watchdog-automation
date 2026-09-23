import { invoke } from "@tauri-apps/api/core";

export interface RelatedFolder {
  key: string;
  name: string;
  purpose: string;
  path: string;
}

export const listRelatedFolders = () => invoke<RelatedFolder[]>("list_related_folders");
export const openRelatedFolder = (key: string) => invoke<void>("open_related_folder", { key });
