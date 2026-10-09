import assert from 'node:assert/strict';
import test from 'node:test';
import {tagsPage,foldersPage,filesPage,cursorQuery} from '../../../web/react/tags-pages.js';
const tag=i=>({id:i,name:`tag-${i}`,color:null,file_count:0});
const page=tags=>({tags,previous_cursor:null,next_cursor:null});
test('tag pages require inverse cursors, unique identities and fifty bounded rows',()=>{
 assert.equal(tagsPage(page(Array.from({length:50},(_,i)=>tag(i+1)))),true);
 assert.equal(tagsPage(page(Array.from({length:51},(_,i)=>tag(i+1)))),false);
 assert.equal(tagsPage(page([tag(1),tag(1)])),false);
 assert.equal(tagsPage({tags:[]}),false);
 assert.equal(tagsPage({...page([]),next_cursor:'a'.repeat(4097)}),false);
 assert.equal(tagsPage(page([{...tag(1),name:'界'.repeat(107)}])),false);
 assert.equal(tagsPage(page([{...tag(1),color:'invalid'}])),false);
 assert.equal(cursorQuery('opaque_safe-1'),'?cursor=opaque_safe-1');
});
test('folders and file badges stay bounded while complete relations have a separate page',()=>{
 assert.equal(foldersPage({folders:['中文','folder'],previous_cursor:null,next_cursor:'cursor'}),true);
 assert.equal(foldersPage({folders:['folder','folder'],previous_cursor:null,next_cursor:null}),false);
 assert.equal(foldersPage({folders:['parent/child'],previous_cursor:null,next_cursor:null}),false);
 const file={id:1,path:'folder/file',name:'file',status:'present',size:1,mtime_ns:1,tag_ids:[1],tag_ids_has_more:true};
 const files={files:[file],total:1,page:1,page_size:50};
 assert.equal(filesPage(files),true);
 assert.equal(filesPage({...files,files:[{...file,tag_ids:Array(51).fill(1)}]}),false);
 assert.equal(filesPage({...files,files:[{...file,tag_ids_has_more:undefined}]}),false);
});
