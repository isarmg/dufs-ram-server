export const PAGE_SIZE = 50;
export const MAX_CURSOR_BYTES = 4096;
/** @typedef {{id:number,name:string,color:string|null,file_count:number}} TagRow */
/** @typedef {{previous_cursor:string|null,next_cursor:string|null,tags:TagRow[]}} TagPage */
/** @typedef {{previous_cursor:string|null,next_cursor:string|null,folders:string[]}} FolderPage */
/** @param {unknown} value @returns {value is Record<string,unknown>} */
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
/** @param {unknown} value @param {number} max @returns {value is string} */
const text = (value, max) => typeof value === 'string' && new TextEncoder().encode(value).byteLength <= max;
/** @param {unknown} value @returns {value is number} */
const integer = value => typeof value === 'number' && Number.isSafeInteger(value);
/** @param {unknown} value @returns {value is number} */
const finite = value => typeof value === 'number' && Number.isFinite(value);
/** @param {unknown} value */
const cursor = value => value === null || (text(value, MAX_CURSOR_BYTES) && /^[A-Za-z0-9_-]+$/.test(value));
/** @param {unknown} value */
const page = value => object(value) && cursor(value.previous_cursor) && cursor(value.next_cursor);
/** @param {unknown} value @returns {value is TagRow} */
export const tagRow = value => object(value) && integer(value.id) && value.id > 0 && text(value.name, 320) && (value.color === null || (text(value.color,7) && /^#[a-fA-F0-9]{6}$/.test(value.color))) && integer(value.file_count) && value.file_count >= 0;
/** @param {unknown} value @returns {value is TagPage} */
export const tagsPage = value => object(value) && page(value) && Array.isArray(value.tags) && value.tags.length <= PAGE_SIZE && value.tags.every(tagRow) && new Set(value.tags.map(tag => tag.id)).size === value.tags.length;
/** @param {unknown} value @returns {value is FolderPage} */
export const foldersPage = value => object(value) && page(value) && Array.isArray(value.folders) && value.folders.length <= PAGE_SIZE && value.folders.every(name => text(name, 768) && name.length > 0 && !name.includes('/')) && new Set(value.folders).size === value.folders.length;
/** @param {unknown} value */
export const filesPage = value => object(value) && Array.isArray(value.files) && value.files.length <= 100 && integer(value.total) && value.total >= 0 && integer(value.page) && value.page > 0 && integer(value.page_size) && value.page_size >= 1 && value.page_size <= 100 && value.files.length <= value.page_size && value.files.every(file => object(file) && integer(file.id) && file.id > 0 && text(file.path, 4096) && text(file.name, 768) && typeof file.status === 'string' && ['present', 'missing', 'suspect'].includes(file.status) && finite(file.size) && file.size >= 0 && finite(file.mtime_ns) && Array.isArray(file.tag_ids) && file.tag_ids.length <= PAGE_SIZE && file.tag_ids.every(id => integer(id) && id > 0) && typeof file.tag_ids_has_more === 'boolean');
/** @param {string|null} value */
export const cursorQuery = value => value === null ? '' : `?${new URLSearchParams({cursor:value})}`;
