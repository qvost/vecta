const section = (name, base, head) => ({name,base,head,delta:head-base});
export const demo = {
  diff: {schema_version:1,format:'Elf',architecture:'X86_64',file:section('file',1843200,2156544),
    sections:[section('.text',1048576,1310720),section('.rodata',393216,458752),section('.data',32768,36864),section('.bss',65536,49152)],
    symbols:[['crypto::handshake::negotiate','.text',65536,229376],['alloc::vec::Vec<T>::extend','.text',32768,114688],['http::routes::dispatch','.text',16384,32768],['lookup::STATIC_TABLE','.rodata',32768,98304],['parser::legacy::decode','.text',24576,8192]].map(([name,section,base,head])=>({...sectionRow(name,base,head),raw_name:name,section,kind:'modified'})),
    libraries_added:['libssl.so.3'],libraries_removed:[],abi:[{family:'GLIBC',base:'2.28',head:'2.34',increased:true},{family:'GLIBCXX',base:'3.4.26',head:'3.4.26',increased:false}],abi_complete:true,warnings:[]},
  gate:{passed:false,violations:['File growth exceeds the 256 KiB budget.','GLIBC 2.34 exceeds maximum 2.28.','New dynamic library: libssl.so.3']},omitted_symbols:0,omitted_sections:0
};
function sectionRow(name,base,head){return {name,base,head,delta:head-base};}
