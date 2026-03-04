Repository in gix - Rustif(window.location.protocol!=="file:")document.head.insertAdjacentHTML("beforeend","SourceSerif4-Regular-6b053e98.ttf.woff2,FiraSans-Italic-81dc35de.woff2,FiraSans-Regular-0fe48ade.woff2,FiraSans-MediumItalic-ccf7e434.woff2,FiraSans-Medium-e1aa3f0a.woff2,SourceCodePro-Regular-8badfe75.ttf.woff2,SourceCodePro-Semibold-aa29a496.ttf.woff2".split(",").map(f=>\`<link rel="preload" as="font" type="font/woff2" crossorigin href="/-/rustdoc.static/${f}">\`).join(""))"use strict";const builtinThemes=\["light","dark","ayu"\];const darkThemes=\["dark","ayu"\];window.currentTheme=(function(){const currentTheme=document.getElementById("themeStyle");return currentTheme instanceof HTMLLinkElement?currentTheme:null;})();const settingsDataset=(function(){const settingsElement=document.getElementById("default-settings");return settingsElement&&settingsElement.dataset?settingsElement.dataset:null;})();function nonnull(x,msg){if(x===null){throw(msg||"unexpected null value!");}else{return x;}}function nonundef(x,msg){if(x===undefined){throw(msg||"unexpected null value!");}else{return x;}}function getSettingValue(settingName){const current=getCurrentValue(settingName);if(current===null&&settingsDataset!==null){const def=settingsDataset\[settingName.replace(/-/g,"\_")\];if(def!==undefined){return def;}}return current;}const localStoredTheme=getSettingValue("theme");function hasClass(elem,className){return!!elem&&!!elem.classList&&elem.classList.contains(className);}function addClass(elem,className){if(elem&&elem.classList){elem.classList.add(className);}}function removeClass(elem,className){if(elem&&elem.classList){elem.classList.remove(className);}}function onEach(arr,func){for(const elem of arr){if(func(elem)){return true;}}return false;}function onEachLazy(lazyArray,func){return onEach(Array.prototype.slice.call(lazyArray),func);}function updateLocalStorage(name,value){try{if(value===null){window.localStorage.removeItem("rustdoc-"+name);}else{window.localStorage.setItem("rustdoc-"+name,value);}}catch(e){}}function getCurrentValue(name){try{return window.localStorage.getItem("rustdoc-"+name);}catch(e){return null;}}function getVar(name){const el=document.querySelector("head > meta\[name='rustdoc-vars'\]");return el?el.getAttribute("data-"+name):null;}function switchTheme(newThemeName,saveTheme){const themeNames=(getVar("themes")||"").split(",").filter(t=>t);themeNames.push(...builtinThemes);if(newThemeName===null||themeNames.indexOf(newThemeName)===-1){return;}if(saveTheme){updateLocalStorage("theme",newThemeName);}document.documentElement.setAttribute("data-theme",newThemeName);if(builtinThemes.indexOf(newThemeName)!==-1){if(window.currentTheme&&window.currentTheme.parentNode){window.currentTheme.parentNode.removeChild(window.currentTheme);window.currentTheme=null;}}else{const newHref=getVar("root-path")+encodeURIComponent(newThemeName)+getVar("resource-suffix")+".css";if(!window.currentTheme){if(document.readyState==="loading"){document.write(\`<link rel="stylesheet" id="themeStyle" href="${newHref}">\`);window.currentTheme=(function(){const currentTheme=document.getElementById("themeStyle");return currentTheme instanceof HTMLLinkElement?currentTheme:null;})();}else{window.currentTheme=document.createElement("link");window.currentTheme.rel="stylesheet";window.currentTheme.id="themeStyle";window.currentTheme.href=newHref;document.documentElement.appendChild(window.currentTheme);}}else if(newHref!==window.currentTheme.href){window.currentTheme.href=newHref;}}}const updateTheme=(function(){const mql=window.matchMedia("(prefers-color-scheme: dark)");function updateTheme(){if(getSettingValue("use-system-theme")!=="false"){const lightTheme=getSettingValue("preferred-light-theme")||"light";const darkTheme=getSettingValue("preferred-dark-theme")||"dark";updateLocalStorage("use-system-theme","true");switchTheme(mql.matches?darkTheme:lightTheme,true);}else{switchTheme(getSettingValue("theme"),false);}}mql.addEventListener("change",updateTheme);return updateTheme;})();if(getSettingValue("use-system-theme")!=="false"&&window.matchMedia){if(getSettingValue("use-system-theme")===null&&getSettingValue("preferred-dark-theme")===null&&localStoredTheme!==null&&darkThemes.indexOf(localStoredTheme)>=0){updateLocalStorage("preferred-dark-theme",localStoredTheme);}}updateTheme();if(getSettingValue("source-sidebar-show")==="true"){addClass(document.documentElement,"src-sidebar-expanded");}if(getSettingValue("hide-sidebar")==="true"){addClass(document.documentElement,"hide-sidebar");}if(getSettingValue("hide-toc")==="true"){addClass(document.documentElement,"hide-toc");}if(getSettingValue("hide-modnav")==="true"){addClass(document.documentElement,"hide-modnav");}if(getSettingValue("sans-serif-fonts")==="true"){addClass(document.documentElement,"sans-serif");}if(getSettingValue("word-wrap-source-code")==="true"){addClass(document.documentElement,"word-wrap-source-code");}function updateSidebarWidth(){const desktopSidebarWidth=getSettingValue("desktop-sidebar-width");if(desktopSidebarWidth&&desktopSidebarWidth!=="null"){document.documentElement.style.setProperty("--desktop-sidebar-width",desktopSidebarWidth+"px",);}const srcSidebarWidth=getSettingValue("src-sidebar-width");if(srcSidebarWidth&&srcSidebarWidth!=="null"){document.documentElement.style.setProperty("--src-sidebar-width",srcSidebarWidth+"px",);}}updateSidebarWidth();window.addEventListener("pageshow",ev=>{if(ev.persisted){setTimeout(updateTheme,0);setTimeout(updateSidebarWidth,0);}});class RustdocSearchElement extends HTMLElement{constructor(){super();}connectedCallback(){const rootPath=getVar("root-path");const currentCrate=getVar("current-crate");this.innerHTML=\`<nav class="sub"> <form class="search-form"> <span></span> <!-- This empty span is a hacky fix for Safari - See #93184 --> <div id="sidebar-button" tabindex="-1"> <a href="${rootPath}${currentCrate}/all.html" title="show sidebar"></a> </div> <input class="search-input" name="search" aria-label="Run search in the documentation" autocomplete="off" spellcheck="false" placeholder="Type ‘S’ or ‘/’ to search, ‘?’ for more options…" type="search"> </form> </nav>\`;}}window.customElements.define("rustdoc-search",RustdocSearchElement);class RustdocToolbarElement extends HTMLElement{constructor(){super();}connectedCallback(){if(this.firstElementChild){return;}const rootPath=getVar("root-path");this.innerHTML=\` <div id="settings-menu" tabindex="-1"> <a href="${rootPath}settings.html"><span class="label">Settings</span></a> </div> <div id="help-button" tabindex="-1"> <a href="${rootPath}help.html"><span class="label">Help</span></a> </div> <button id="toggle-all-docs"><span class="label">Summary</span></button>\`;}}window.customElements.define("rustdoc-toolbar",RustdocToolbarElement);window.SIDEBAR\_ITEMS = {"enum":\["ObjectId"\],"fn":\["discover","init","init\_bare","open","open\_opts","prepare\_clone","prepare\_clone\_bare"\],"mod":\["clone","commit","config","create","diff","dirwalk","discover","env","filter","head","id","index","init","interrupt","mailmap","merge","object","open","parallel","path","pathspec","prelude","progress","push","reference","remote","repository","revision","shallow","state","status","submodule","tag","threading","worktree"\],"struct":\["AttributeStack","Blob","Commit","Head","Id","Object","ObjectDetached","Pathspec","PathspecDetached","Reference","Remote","Repository","Submodule","Tag","ThreadSafeRepository","Tree","Url","Worktree","oid"\],"trait":\["Count","DynNestedProgress","NestedProgress","Progress"\],"type":\["OdbHandle","OdbHandleArc","RefStore"\]};"use strict";window.RUSTDOC\_TOOLTIP\_HOVER\_MS=300;window.RUSTDOC\_TOOLTIP\_HOVER\_EXIT\_MS=450;function resourcePath(basename,extension){return getVar("root-path")+basename+getVar("resource-suffix")+extension;}function hideMain(){addClass(document.getElementById(MAIN\_ID),"hidden");const toggle=document.getElementById("toggle-all-docs");if(toggle){toggle.setAttribute("disabled","disabled");}}function showMain(){const main=document.getElementById(MAIN\_ID);if(!main){return;}removeClass(main,"hidden");const mainHeading=main.querySelector(".main-heading");if(mainHeading&&window.searchState.rustdocToolbar){if(window.searchState.rustdocToolbar.parentElement){window.searchState.rustdocToolbar.parentElement.removeChild(window.searchState.rustdocToolbar,);}mainHeading.appendChild(window.searchState.rustdocToolbar);}const toggle=document.getElementById("toggle-all-docs");if(toggle){toggle.removeAttribute("disabled");}}window.rootPath=getVar("root-path");window.currentCrate=getVar("current-crate");function setMobileTopbar(){const mobileTopbar=document.querySelector(".mobile-topbar");const locationTitle=document.querySelector(".sidebar h2.location");if(mobileTopbar){const mobileTitle=document.createElement("h2");mobileTitle.className="location";if(hasClass(document.querySelector(".rustdoc"),"crate")){mobileTitle.innerHTML=\`Crate <a href="#">${window.currentCrate}</a>\`;}else if(locationTitle){mobileTitle.innerHTML=locationTitle.innerHTML;}mobileTopbar.appendChild(mobileTitle);}}function getVirtualKey(ev){if("key"in ev&&typeof ev.key!=="undefined"){return ev.key;}const c=ev.charCode||ev.keyCode;if(c===27){return"Escape";}return String.fromCharCode(c);}const MAIN\_ID="main-content";const SETTINGS\_BUTTON\_ID="settings-menu";const ALTERNATIVE\_DISPLAY\_ID="alternative-display";const NOT\_DISPLAYED\_ID="not-displayed";const HELP\_BUTTON\_ID="help-button";function getSettingsButton(){return document.getElementById(SETTINGS\_BUTTON\_ID);}function getHelpButton(){return document.getElementById(HELP\_BUTTON\_ID);}function getNakedUrl(){return window.location.href.split("?")\[0\].split("#")\[0\];}function insertAfter(newNode,referenceNode){referenceNode.parentNode.insertBefore(newNode,referenceNode.nextSibling);}function getOrCreateSection(id,classes){let el=document.getElementById(id);if(!el){el=document.createElement("section");el.id=id;el.className=classes;insertAfter(el,document.getElementById(MAIN\_ID));}return el;}function getAlternativeDisplayElem(){return getOrCreateSection(ALTERNATIVE\_DISPLAY\_ID,"content hidden");}function getNotDisplayedElem(){return getOrCreateSection(NOT\_DISPLAYED\_ID,"hidden");}function switchDisplayedElement(elemToDisplay){const el=getAlternativeDisplayElem();if(el.children.length>0){getNotDisplayedElem().appendChild(el.firstElementChild);}if(elemToDisplay===null){addClass(el,"hidden");showMain();return;}el.appendChild(elemToDisplay);hideMain();removeClass(el,"hidden");const mainHeading=elemToDisplay.querySelector(".main-heading");if(mainHeading&&window.searchState.rustdocToolbar){if(window.searchState.rustdocToolbar.parentElement){window.searchState.rustdocToolbar.parentElement.removeChild(window.searchState.rustdocToolbar,);}mainHeading.appendChild(window.searchState.rustdocToolbar);}}function browserSupportsHistoryApi(){return window.history&&typeof window.history.pushState==="function";}function preLoadCss(cssUrl){const link=document.createElement("link");link.href=cssUrl;link.rel="preload";link.as="style";document.getElementsByTagName("head")\[0\].appendChild(link);}(function(){const isHelpPage=window.location.pathname.endsWith("/help.html");function loadScript(url,errorCallback){const script=document.createElement("script");script.src=url;if(errorCallback!==undefined){script.onerror=errorCallback;}document.head.append(script);}const settingsButton=getSettingsButton();if(settingsButton){settingsButton.onclick=event=>{if(event.ctrlKey||event.altKey||event.metaKey){return;}window.hideAllModals(false);addClass(getSettingsButton(),"rotate");event.preventDefault();loadScript(getVar("static-root-path")+getVar("settings-js"));setTimeout(()=>{const themes=getVar("themes").split(",");for(const theme of themes){if(theme!==""){preLoadCss(getVar("root-path")+theme+".css");}}},0);};}window.searchState={rustdocToolbar:document.querySelector("rustdoc-toolbar"),loadingText:"Loading search results...",input:document.getElementsByClassName("search-input")\[0\],outputElement:()=>{let el=document.getElementById("search");if(!el){el=document.createElement("section");el.id="search";getNotDisplayedElem().appendChild(el);}return el;},title:document.title,titleBeforeSearch:document.title,timeout:null,currentTab:0,focusedByTab:\[null,null,null\],clearInputTimeout:()=>{if(window.searchState.timeout!==null){clearTimeout(window.searchState.timeout);window.searchState.timeout=null;}},isDisplayed:()=>{const outputElement=window.searchState.outputElement();return!!outputElement&&!!outputElement.parentElement&&outputElement.parentElement.id===ALTERNATIVE\_DISPLAY\_ID;},focus:()=>{window.searchState.input&&window.searchState.input.focus();},defocus:()=>{window.searchState.input&&window.searchState.input.blur();},showResults:search=>{if(search===null||typeof search==="undefined"){search=window.searchState.outputElement();}switchDisplayedElement(search);document.title=window.searchState.title;},removeQueryParameters:()=>{document.title=window.searchState.titleBeforeSearch;if(browserSupportsHistoryApi()){history.replaceState(null,"",getNakedUrl()+window.location.hash);}},hideResults:()=>{switchDisplayedElement(null);window.searchState.removeQueryParameters();},getQueryStringParams:()=>{const params={};window.location.search.substring(1).split("&").map(s=>{const pair=s.split("=").map(x=>x.replace(/\\+/g," "));params\[decodeURIComponent(pair\[0\])\]=typeof pair\[1\]==="undefined"?null:decodeURIComponent(pair\[1\]);});return params;},setup:()=>{const search\_input=window.searchState.input;if(!search\_input){return;}let searchLoaded=false;function sendSearchForm(){document.getElementsByClassName("search-form")\[0\].submit();}function loadSearch(){if(!searchLoaded){searchLoaded=true;loadScript(getVar("static-root-path")+getVar("search-js"),sendSearchForm);loadScript(resourcePath("search-index",".js"),sendSearchForm);}}search\_input.addEventListener("focus",()=>{window.searchState.origPlaceholder=search\_input.placeholder;search\_input.placeholder="Type your search here.";loadSearch();});if(search\_input.value!==""){loadSearch();}const params=window.searchState.getQueryStringParams();if(params.search!==undefined){window.searchState.setLoadingSearch();loadSearch();}},setLoadingSearch:()=>{const search=window.searchState.outputElement();if(!search){return;}search.innerHTML="<h3 class=\\"search-loading\\">"+window.searchState.loadingText+"</h3>";window.searchState.showResults(search);},descShards:new Map(),loadDesc:async function({descShard,descIndex}){if(descShard.promise===null){descShard.promise=new Promise((resolve,reject)=>{descShard.resolve=resolve;const ds=descShard;const fname=\`${ds.crate}-desc-${ds.shard}-\`;const url=resourcePath(\`search.desc/${descShard.crate}/${fname}\`,".js",);loadScript(url,reject);});}const list=await descShard.promise;return list\[descIndex\];},loadedDescShard:function(crate,shard,data){this.descShards.get(crate)\[shard\].resolve(data.split("\\n"));},};const toggleAllDocsId="toggle-all-docs";let savedHash="";function handleHashes(ev){if(ev!==null&&window.searchState.isDisplayed()&&ev.newURL){switchDisplayedElement(null);const hash=ev.newURL.slice(ev.newURL.indexOf("#")+1);if(browserSupportsHistoryApi()){history.replaceState(null,"",getNakedUrl()+window.location.search+"#"+hash);}const elem=document.getElementById(hash);if(elem){elem.scrollIntoView();}}const pageId=window.location.hash.replace(/^#/,"");if(savedHash!==pageId){savedHash=pageId;if(pageId!==""){expandSection(pageId);}}if(savedHash.startsWith("impl-")){const splitAt=savedHash.indexOf("/");if(splitAt!==-1){const implId=savedHash.slice(0,splitAt);const assocId=savedHash.slice(splitAt+1);const implElems=document.querySelectorAll(\`details > summary > section\[id^="${implId}"\]\`,);onEachLazy(implElems,implElem=>{const numbered=/^(.+?)-(\[0-9\]+)$/.exec(implElem.id);if(implElem.id!==implId&&(!numbered||numbered\[1\]!==implId)){return false;}return onEachLazy(implElem.parentElement.parentElement.querySelectorAll(\`\[id^="${assocId}"\]\`),item=>{const numbered=/^(.+?)-(\[0-9\]+)$/.exec(item.id);if(item.id===assocId||(numbered&&numbered\[1\]===assocId)){openParentDetails(item);item.scrollIntoView();setTimeout(()=>{window.location.replace("#"+item.id);},0);return true;}},);});}}}function onHashChange(ev){hideSidebar();handleHashes(ev);}function openParentDetails(elem){while(elem){if(elem.tagName==="DETAILS"){elem.open=true;}elem=elem.parentElement;}}function expandSection(id){openParentDetails(document.getElementById(id));}function handleEscape(ev){window.searchState.clearInputTimeout();window.searchState.hideResults();ev.preventDefault();window.searchState.defocus();window.hideAllModals(true);}function handleShortcut(ev){const disableShortcuts=getSettingValue("disable-shortcuts")==="true";if(ev.ctrlKey||ev.altKey||ev.metaKey||disableShortcuts){return;}if(document.activeElement&&document.activeElement.tagName==="INPUT"&&document.activeElement.type!=="checkbox"&&document.activeElement.type!=="radio"){switch(getVirtualKey(ev)){case"Escape":handleEscape(ev);break;}}else{switch(getVirtualKey(ev)){case"Escape":handleEscape(ev);break;case"s":case"S":case"/":ev.preventDefault();window.searchState.focus();break;case"+":ev.preventDefault();expandAllDocs();break;case"-":ev.preventDefault();collapseAllDocs();break;case"?":showHelp();break;default:break;}}}document.addEventListener("keypress",handleShortcut);document.addEventListener("keydown",handleShortcut);function addSidebarItems(){if(!window.SIDEBAR\_ITEMS){return;}const sidebar=document.getElementById("rustdoc-modnav");function block(shortty,id,longty){const filtered=window.SIDEBAR\_ITEMS\[shortty\];if(!filtered){return;}const modpath=hasClass(document.querySelector(".rustdoc"),"mod")?"../":"";const h3=document.createElement("h3");h3.innerHTML=\`<a href="${modpath}index.html#${id}">${longty}</a>\`;const ul=document.createElement("ul");ul.className="block "+shortty;for(const name of filtered){let path;if(shortty==="mod"){path=\`${modpath}${name}/index.html\`;}else{path=\`${modpath}${shortty}.${name}.html\`;}let current\_page=document.location.href.toString();if(current\_page.endsWith("/")){current\_page+="index.html";}const link=document.createElement("a");link.href=path;link.textContent=name;const li=document.createElement("li");if(link.href===current\_page){li.classList.add("current");}li.appendChild(link);ul.appendChild(li);}sidebar.appendChild(h3);sidebar.appendChild(ul);}if(sidebar){block("primitive","primitives","Primitive Types");block("mod","modules","Modules");block("macro","macros","Macros");block("struct","structs","Structs");block("enum","enums","Enums");block("constant","constants","Constants");block("static","static","Statics");block("trait","traits","Traits");block("fn","functions","Functions");block("type","types","Type Aliases");block("union","unions","Unions");block("foreigntype","foreign-types","Foreign Types");block("keyword","keywords","Keywords");block("attr","attributes","Attribute Macros");block("derive","derives","Derive Macros");block("traitalias","trait-aliases","Trait Aliases");}}window.register\_implementors=imp=>{const implementors=document.getElementById("implementors-list");const synthetic\_implementors=document.getElementById("synthetic-implementors-list");const inlined\_types=new Set();const TEXT\_IDX=0;const SYNTHETIC\_IDX=1;const TYPES\_IDX=2;if(synthetic\_implementors){onEachLazy(synthetic\_implementors.getElementsByClassName("impl"),el=>{const aliases=el.getAttribute("data-aliases");if(!aliases){return;}aliases.split(",").forEach(alias=>{inlined\_types.add(alias);});});}let currentNbImpls=implementors.getElementsByClassName("impl").length;const traitName=document.querySelector(".main-heading h1 > .trait").textContent;const baseIdName="impl-"+traitName+"-";const libs=Object.getOwnPropertyNames(imp);const script=document.querySelector("script\[data-ignore-extern-crates\]");const ignoreExternCrates=new Set((script?script.getAttribute("data-ignore-extern-crates"):"").split(","),);for(const lib of libs){if(lib===window.currentCrate||ignoreExternCrates.has(lib)){continue;}const structs=imp\[lib\];struct\_loop:for(const struct of structs){const list=struct\[SYNTHETIC\_IDX\]?synthetic\_implementors:implementors;if(struct\[SYNTHETIC\_IDX\]){for(const struct\_type of struct\[TYPES\_IDX\]){if(inlined\_types.has(struct\_type)){continue struct\_loop;}inlined\_types.add(struct\_type);}}const code=document.createElement("h3");code.innerHTML=struct\[TEXT\_IDX\];addClass(code,"code-header");onEachLazy(code.getElementsByTagName("a"),elem=>{const href=elem.getAttribute("href");if(href&&!href.startsWith("#")&&!/^(?:\[a-z+\]+:)?\\/\\//.test(href)){elem.setAttribute("href",window.rootPath+href);}});const currentId=baseIdName+currentNbImpls;const anchor=document.createElement("a");anchor.href="#"+currentId;addClass(anchor,"anchor");const display=document.createElement("div");display.id=currentId;addClass(display,"impl");display.appendChild(anchor);display.appendChild(code);list.appendChild(display);currentNbImpls+=1;}}};if(window.pending\_implementors){window.register\_implementors(window.pending\_implementors);}window.register\_type\_impls=imp=>{if(!imp||!imp\[window.currentCrate\]){return;}window.pending\_type\_impls=undefined;const idMap=new Map();let implementations=document.getElementById("implementations-list");let trait\_implementations=document.getElementById("trait-implementations-list");let trait\_implementations\_header=document.getElementById("trait-implementations");const script=document.querySelector("script\[data-self-path\]");const selfPath=script?script.getAttribute("data-self-path"):null;const mainContent=document.querySelector("#main-content");const sidebarSection=document.querySelector(".sidebar section");let methods=document.querySelector(".sidebar .block.method");let associatedTypes=document.querySelector(".sidebar .block.associatedtype");let associatedConstants=document.querySelector(".sidebar .block.associatedconstant");let sidebarTraitList=document.querySelector(".sidebar .block.trait-implementation");for(const impList of imp\[window.currentCrate\]){const types=impList.slice(2);const text=impList\[0\];const isTrait=impList\[1\]!==0;const traitName=impList\[1\];if(types.indexOf(selfPath)===-1){continue;}let outputList=isTrait?trait\_implementations:implementations;if(outputList===null){const outputListName=isTrait?"Trait Implementations":"Implementations";const outputListId=isTrait?"trait-implementations-list":"implementations-list";const outputListHeaderId=isTrait?"trait-implementations":"implementations";const outputListHeader=document.createElement("h2");outputListHeader.id=outputListHeaderId;outputListHeader.innerText=outputListName;outputList=document.createElement("div");outputList.id=outputListId;if(isTrait){const link=document.createElement("a");link.href=\`#${outputListHeaderId}\`;link.innerText="Trait Implementations";const h=document.createElement("h3");h.appendChild(link);trait\_implementations=outputList;trait\_implementations\_header=outputListHeader;sidebarSection.appendChild(h);sidebarTraitList=document.createElement("ul");sidebarTraitList.className="block trait-implementation";sidebarSection.appendChild(sidebarTraitList);mainContent.appendChild(outputListHeader);mainContent.appendChild(outputList);}else{implementations=outputList;if(trait\_implementations){mainContent.insertBefore(outputListHeader,trait\_implementations\_header);mainContent.insertBefore(outputList,trait\_implementations\_header);}else{const mainContent=document.querySelector("#main-content");mainContent.appendChild(outputListHeader);mainContent.appendChild(outputList);}}}const template=document.createElement("template");template.innerHTML=text;onEachLazy(template.content.querySelectorAll("a"),elem=>{const href=elem.getAttribute("href");if(href&&!href.startsWith("#")&&!/^(?:\[a-z+\]+:)?\\/\\//.test(href)){elem.setAttribute("href",window.rootPath+href);}});onEachLazy(template.content.querySelectorAll("\[id\]"),el=>{let i=0;if(idMap.has(el.id)){i=idMap.get(el.id);}else if(document.getElementById(el.id)){i=1;while(document.getElementById(\`${el.id}-${2 \* i}\`)){i=2\*i;}while(document.getElementById(\`${el.id}-${i}\`)){i+=1;}}if(i!==0){const oldHref=\`#${el.id}\`;const newHref=\`#${el.id}-${i}\`;el.id=\`${el.id}-${i}\`;onEachLazy(template.content.querySelectorAll("a\[href\]"),link=>{if(link.getAttribute("href")===oldHref){link.href=newHref;}});}idMap.set(el.id,i+1);});const templateAssocItems=template.content.querySelectorAll("section.tymethod, "+"section.method, section.associatedtype, section.associatedconstant");if(isTrait){const li=document.createElement("li");const a=document.createElement("a");a.href=\`#${template.content.querySelector(".impl").id}\`;a.textContent=traitName;li.appendChild(a);sidebarTraitList.append(li);}else{onEachLazy(templateAssocItems,item=>{let block=hasClass(item,"associatedtype")?associatedTypes:(hasClass(item,"associatedconstant")?associatedConstants:(methods));if(!block){const blockTitle=hasClass(item,"associatedtype")?"Associated Types":(hasClass(item,"associatedconstant")?"Associated Constants":("Methods"));const blockClass=hasClass(item,"associatedtype")?"associatedtype":(hasClass(item,"associatedconstant")?"associatedconstant":("method"));const blockHeader=document.createElement("h3");const blockLink=document.createElement("a");blockLink.href="#implementations";blockLink.innerText=blockTitle;blockHeader.appendChild(blockLink);block=document.createElement("ul");block.className=\`block ${blockClass}\`;const insertionReference=methods||sidebarTraitList;if(insertionReference){const insertionReferenceH=insertionReference.previousElementSibling;sidebarSection.insertBefore(blockHeader,insertionReferenceH);sidebarSection.insertBefore(block,insertionReferenceH);}else{sidebarSection.appendChild(blockHeader);sidebarSection.appendChild(block);}if(hasClass(item,"associatedtype")){associatedTypes=block;}else if(hasClass(item,"associatedconstant")){associatedConstants=block;}else{methods=block;}}const li=document.createElement("li");const a=document.createElement("a");a.innerText=item.id.split("-")\[0\].split(".")\[1\];a.href=\`#${item.id}\`;li.appendChild(a);block.appendChild(li);});}outputList.appendChild(template.content);}for(const list of\[methods,associatedTypes,associatedConstants,sidebarTraitList\]){if(!list){continue;}const newChildren=Array.prototype.slice.call(list.children);newChildren.sort((a,b)=>{const aI=a.innerText;const bI=b.innerText;return aI<bI?-1:aI>bI?1:0;});list.replaceChildren(...newChildren);}};if(window.pending\_type\_impls){window.register\_type\_impls(window.pending\_type\_impls);}function addSidebarCrates(){if(!window.ALL\_CRATES){return;}const sidebarElems=document.getElementById("rustdoc-modnav");if(!sidebarElems){return;}const h3=document.createElement("h3");h3.innerHTML="Crates";const ul=document.createElement("ul");ul.className="block crate";for(const crate of window.ALL\_CRATES){const link=document.createElement("a");link.href=window.rootPath+crate+"/index.html";link.textContent=crate;const li=document.createElement("li");if(window.rootPath!=="./"&&crate===window.currentCrate){li.className="current";}li.appendChild(link);ul.appendChild(li);}sidebarElems.appendChild(h3);sidebarElems.appendChild(ul);}function expandAllDocs(){const innerToggle=document.getElementById(toggleAllDocsId);removeClass(innerToggle,"will-expand");onEachLazy(document.getElementsByClassName("toggle"),e=>{if(!hasClass(e,"type-contents-toggle")&&!hasClass(e,"more-examples-toggle")){e.open=true;}});innerToggle.children\[0\].innerText="Summary";}function collapseAllDocs(){const innerToggle=document.getElementById(toggleAllDocsId);addClass(innerToggle,"will-expand");onEachLazy(document.getElementsByClassName("toggle"),e=>{if(e.parentNode.id!=="implementations-list"||(!hasClass(e,"implementors-toggle")&&!hasClass(e,"type-contents-toggle"))){e.open=false;}});innerToggle.children\[0\].innerText="Show all";}function toggleAllDocs(){const innerToggle=document.getElementById(toggleAllDocsId);if(!innerToggle){return;}if(hasClass(innerToggle,"will-expand")){expandAllDocs();}else{collapseAllDocs();}}(function(){const toggles=document.getElementById(toggleAllDocsId);if(toggles){toggles.onclick=toggleAllDocs;}const hideMethodDocs=getSettingValue("auto-hide-method-docs")==="true";const hideImplementations=getSettingValue("auto-hide-trait-implementations")==="true";const hideLargeItemContents=getSettingValue("auto-hide-large-items")!=="false";function setImplementorsTogglesOpen(id,open){const list=document.getElementById(id);if(list!==null){onEachLazy(list.getElementsByClassName("implementors-toggle"),e=>{e.open=open;});}}if(hideImplementations){setImplementorsTogglesOpen("trait-implementations-list",false);setImplementorsTogglesOpen("blanket-implementations-list",false);}onEachLazy(document.getElementsByClassName("toggle"),e=>{if(!hideLargeItemContents&&hasClass(e,"type-contents-toggle")){e.open=true;}if(hideMethodDocs&&hasClass(e,"method-toggle")){e.open=false;}});}());window.rustdoc\_add\_line\_numbers\_to\_examples=()=>{function generateLine(nb){return\`<span data-nosnippet>${nb}</span>\`;}onEachLazy(document.querySelectorAll(".rustdoc:not(.src) :not(.scraped-example) > .example-wrap > pre > code",),code=>{if(hasClass(code.parentElement.parentElement,"hide-lines")){removeClass(code.parentElement.parentElement,"hide-lines");return;}const lines=code.innerHTML.split("\\n");const digits=(lines.length+"").length;code.innerHTML=lines.map((line,index)=>generateLine(index+1)+line).join("\\n");addClass(code.parentElement.parentElement,\`digits-${digits}\`);});};window.rustdoc\_remove\_line\_numbers\_from\_examples=()=>{onEachLazy(document.querySelectorAll(".rustdoc:not(.src) :not(.scraped-example) > .example-wrap"),x=>addClass(x,"hide-lines"),);};if(getSettingValue("line-numbers")==="true"){window.rustdoc\_add\_line\_numbers\_to\_examples();}function showSidebar(){window.hideAllModals(false);const sidebar=document.getElementsByClassName("sidebar")\[0\];addClass(sidebar,"shown");}function hideSidebar(){const sidebar=document.getElementsByClassName("sidebar")\[0\];removeClass(sidebar,"shown");}window.addEventListener("resize",()=>{if(window.CURRENT\_TOOLTIP\_ELEMENT){const base=window.CURRENT\_TOOLTIP\_ELEMENT.TOOLTIP\_BASE;const force\_visible=base.TOOLTIP\_FORCE\_VISIBLE;hideTooltip(false);if(force\_visible){showTooltip(base);base.TOOLTIP\_FORCE\_VISIBLE=true;}}});const mainElem=document.getElementById(MAIN\_ID);if(mainElem){mainElem.addEventListener("click",hideSidebar);}onEachLazy(document.querySelectorAll("a\[href^='#'\]"),el=>{el.addEventListener("click",()=>{expandSection(el.hash.slice(1));hideSidebar();});});onEachLazy(document.querySelectorAll(".toggle > summary:not(.hideme)"),el=>{el.addEventListener("click",e=>{if(!e.target.matches("summary, a, a \*")){e.preventDefault();}});});function showTooltip(e){const notable\_ty=e.getAttribute("data-notable-ty");if(!window.NOTABLE\_TRAITS&&notable\_ty){const data=document.getElementById("notable-traits-data");if(data){window.NOTABLE\_TRAITS=JSON.parse(data.innerText);}else{throw new Error("showTooltip() called with notable without any notable traits!");}}if(window.CURRENT\_TOOLTIP\_ELEMENT&&window.CURRENT\_TOOLTIP\_ELEMENT.TOOLTIP\_BASE===e){clearTooltipHoverTimeout(window.CURRENT\_TOOLTIP\_ELEMENT);return;}window.hideAllModals(false);const wrapper=document.createElement("div");if(notable\_ty){wrapper.innerHTML="<div class=\\"content\\">"+window.NOTABLE\_TRAITS\[notable\_ty\]+"</div>";}else{const ttl=e.getAttribute("title");if(ttl!==null){e.setAttribute("data-title",ttl);e.removeAttribute("title");}const dttl=e.getAttribute("data-title");if(dttl!==null){const titleContent=document.createElement("div");titleContent.className="content";titleContent.appendChild(document.createTextNode(dttl));wrapper.appendChild(titleContent);}}wrapper.className="tooltip popover";const focusCatcher=document.createElement("div");focusCatcher.setAttribute("tabindex","0");focusCatcher.onfocus=hideTooltip;wrapper.appendChild(focusCatcher);const pos=e.getBoundingClientRect();wrapper.style.top=(pos.top+window.scrollY+pos.height)+"px";wrapper.style.left=0;wrapper.style.right="auto";wrapper.style.visibility="hidden";document.body.appendChild(wrapper);const wrapperPos=wrapper.getBoundingClientRect();const finalPos=pos.left+window.scrollX-wrapperPos.width+24;if(finalPos>0){wrapper.style.left=finalPos+"px";}else{wrapper.style.setProperty("--popover-arrow-offset",(wrapperPos.right-pos.right+4)+"px",);}wrapper.style.visibility="";window.CURRENT\_TOOLTIP\_ELEMENT=wrapper;window.CURRENT\_TOOLTIP\_ELEMENT.TOOLTIP\_BASE=e;clearTooltipHoverTimeout(window.CURRENT\_TOOLTIP\_ELEMENT);wrapper.onpointerenter=ev=>{if(ev.pointerType!=="mouse"){return;}clearTooltipHoverTimeout(e);};wrapper.onpointerleave=ev=>{if(ev.pointerType!=="mouse"||!(ev.relatedTarget instanceof HTMLElement)){return;}if(!e.TOOLTIP\_FORCE\_VISIBLE&&!e.contains(ev.relatedTarget)){setTooltipHoverTimeout(e,false);addClass(wrapper,"fade-out");}};}function setTooltipHoverTimeout(element,show){clearTooltipHoverTimeout(element);if(!show&&!window.CURRENT\_TOOLTIP\_ELEMENT){return;}if(show&&window.CURRENT\_TOOLTIP\_ELEMENT){return;}if(window.CURRENT\_TOOLTIP\_ELEMENT&&window.CURRENT\_TOOLTIP\_ELEMENT.TOOLTIP\_BASE!==element){return;}element.TOOLTIP\_HOVER\_TIMEOUT=setTimeout(()=>{if(show){showTooltip(element);}else if(!element.TOOLTIP\_FORCE\_VISIBLE){hideTooltip(false);}},show?window.RUSTDOC\_TOOLTIP\_HOVER\_MS:window.RUSTDOC\_TOOLTIP\_HOVER\_EXIT\_MS);}function clearTooltipHoverTimeout(element){if(element.TOOLTIP\_HOVER\_TIMEOUT!==undefined){removeClass(window.CURRENT\_TOOLTIP\_ELEMENT,"fade-out");clearTimeout(element.TOOLTIP\_HOVER\_TIMEOUT);delete element.TOOLTIP\_HOVER\_TIMEOUT;}}function tooltipBlurHandler(event){if(window.CURRENT\_TOOLTIP\_ELEMENT&&!window.CURRENT\_TOOLTIP\_ELEMENT.contains(document.activeElement)&&!window.CURRENT\_TOOLTIP\_ELEMENT.contains(event.relatedTarget)&&!window.CURRENT\_TOOLTIP\_ELEMENT.TOOLTIP\_BASE.contains(document.activeElement)&&!window.CURRENT\_TOOLTIP\_ELEMENT.TOOLTIP\_BASE.contains(event.relatedTarget)){setTimeout(()=>hideTooltip(false),0);}}function hideTooltip(focus){if(window.CURRENT\_TOOLTIP\_ELEMENT){if(window.CURRENT\_TOOLTIP\_ELEMENT.TOOLTIP\_BASE.TOOLTIP\_FORCE\_VISIBLE){if(focus){window.CURRENT\_TOOLTIP\_ELEMENT.TOOLTIP\_BASE.focus();}window.CURRENT\_TOOLTIP\_ELEMENT.TOOLTIP\_BASE.TOOLTIP\_FORCE\_VISIBLE=false;}document.body.removeChild(window.CURRENT\_TOOLTIP\_ELEMENT);clearTooltipHoverTimeout(window.CURRENT\_TOOLTIP\_ELEMENT);window.CURRENT\_TOOLTIP\_ELEMENT=null;}}onEachLazy(document.getElementsByClassName("tooltip"),e=>{e.onclick=()=>{e.TOOLTIP\_FORCE\_VISIBLE=e.TOOLTIP\_FORCE\_VISIBLE?false:true;if(window.CURRENT\_TOOLTIP\_ELEMENT&&!e.TOOLTIP\_FORCE\_VISIBLE){hideTooltip(true);}else{showTooltip(e);window.CURRENT\_TOOLTIP\_ELEMENT.setAttribute("tabindex","0");window.CURRENT\_TOOLTIP\_ELEMENT.focus();window.CURRENT\_TOOLTIP\_ELEMENT.onblur=tooltipBlurHandler;}return false;};e.onpointerenter=ev=>{if(ev.pointerType!=="mouse"){return;}setTooltipHoverTimeout(e,true);};e.onpointermove=ev=>{if(ev.pointerType!=="mouse"){return;}setTooltipHoverTimeout(e,true);};e.onpointerleave=ev=>{if(ev.pointerType!=="mouse"){return;}if(!e.TOOLTIP\_FORCE\_VISIBLE&&window.CURRENT\_TOOLTIP\_ELEMENT&&!window.CURRENT\_TOOLTIP\_ELEMENT.contains(ev.relatedTarget)){setTooltipHoverTimeout(e,false);addClass(window.CURRENT\_TOOLTIP\_ELEMENT,"fade-out");}};});const sidebar\_menu\_toggle=document.getElementsByClassName("sidebar-menu-toggle")\[0\];if(sidebar\_menu\_toggle){sidebar\_menu\_toggle.addEventListener("click",()=>{const sidebar=document.getElementsByClassName("sidebar")\[0\];if(!hasClass(sidebar,"shown")){showSidebar();}else{hideSidebar();}});}function helpBlurHandler(event){if(!getHelpButton().contains(document.activeElement)&&!getHelpButton().contains(event.relatedTarget)&&!getSettingsButton().contains(document.activeElement)&&!getSettingsButton().contains(event.relatedTarget)){window.hidePopoverMenus();}}function buildHelpMenu(){const book\_info=document.createElement("span");const drloChannel=\`https://doc.rust-lang.org/${getVar("channel")}\`;book\_info.className="top";book\_info.innerHTML=\`You can find more information in \\ <a href="${drloChannel}/rustdoc/">the rustdoc book</a>.\`;const shortcuts=\[\["?","Show this help dialog"\],\["S / /","Focus the search field"\],\["↑","Move up in search results"\],\["↓","Move down in search results"\],\["← / →","Switch result tab (when results focused)"\],\["&#9166;","Go to active search result"\],\["+","Expand all sections"\],\["-","Collapse all sections"\],\].map(x=>"<dt>"+x\[0\].split(" ").map((y,index)=>((index&1)===0?"<kbd>"+y+"</kbd>":" "+y+" ")).join("")+"</dt><dd>"+x\[1\]+"</dd>").join("");const div\_shortcuts=document.createElement("div");addClass(div\_shortcuts,"shortcuts");div\_shortcuts.innerHTML="<h2>Keyboard Shortcuts</h2><dl>"+shortcuts+"</dl></div>";const infos=\[\`For a full list of all search features, take a look \\ <a href="${drloChannel}/rustdoc/read-documentation/search.html">here</a>.\`,"Prefix searches with a type followed by a colon (e.g., <code>fn:</code>) to \\ restrict the search to a given item kind.","Accepted kinds are: <code>fn</code>, <code>mod</code>, <code>struct</code>, \\ <code>enum</code>, <code>trait</code>, <code>type</code>, <code>macro</code>, \\ and <code>const</code>.","Search functions by type signature (e.g., <code>vec -&gt; usize</code> or \\ <code>-&gt; vec</code> or <code>String, enum:Cow -&gt; bool</code>)","You can look for items with an exact name by putting double quotes around \\ your request: <code>\\"string\\"</code>",\`Look for functions that accept or return \\ <a href="${drloChannel}/std/primitive.slice.html">slices</a> and \\ <a href="${drloChannel}/std/primitive.array.html">arrays</a> by writing square \\ brackets (e.g., <code>-&gt; \[u8\]</code> or <code>\[\] -&gt; Option</code>)\`,"Look for items inside another one by searching for a path: <code>vec::Vec</code>",\].map(x=>"<p>"+x+"</p>").join("");const div\_infos=document.createElement("div");addClass(div\_infos,"infos");div\_infos.innerHTML="<h2>Search Tricks</h2>"+infos;const rustdoc\_version=document.createElement("span");rustdoc\_version.className="bottom";const rustdoc\_version\_code=document.createElement("code");rustdoc\_version\_code.innerText="rustdoc "+getVar("rustdoc-version");rustdoc\_version.appendChild(rustdoc\_version\_code);const container=document.createElement("div");if(!isHelpPage){container.className="popover";}container.id="help";container.style.display="none";const side\_by\_side=document.createElement("div");side\_by\_side.className="side-by-side";side\_by\_side.appendChild(div\_shortcuts);side\_by\_side.appendChild(div\_infos);container.appendChild(book\_info);container.appendChild(side\_by\_side);container.appendChild(rustdoc\_version);if(isHelpPage){const help\_section=document.createElement("section");help\_section.appendChild(container);document.getElementById("main-content").appendChild(help\_section);container.style.display="block";}else{const help\_button=getHelpButton();help\_button.appendChild(container);container.onblur=helpBlurHandler;help\_button.onblur=helpBlurHandler;help\_button.children\[0\].onblur=helpBlurHandler;}return container;}window.hideAllModals=switchFocus=>{hideSidebar();window.hidePopoverMenus();hideTooltip(switchFocus);};window.hidePopoverMenus=()=>{onEachLazy(document.querySelectorAll("rustdoc-toolbar .popover"),elem=>{elem.style.display="none";});const button=getHelpButton();if(button){removeClass(button,"help-open");}};function getHelpMenu(buildNeeded){let menu=getHelpButton().querySelector(".popover");if(!menu&&buildNeeded){menu=buildHelpMenu();}return menu;}function showHelp(){const button=getHelpButton();addClass(button,"help-open");button.querySelector("a").focus();const menu=getHelpMenu(true);if(menu.style.display==="none"){window.hideAllModals();menu.style.display="";}}const helpLink=document.querySelector(\`#${HELP\_BUTTON\_ID} > a\`);if(isHelpPage){buildHelpMenu();}else if(helpLink){helpLink.addEventListener("click",event=>{if(!helpLink.contains(helpLink)||event.ctrlKey||event.altKey||event.metaKey){return;}event.preventDefault();const menu=getHelpMenu(true);const shouldShowHelp=menu.style.display==="none";if(shouldShowHelp){showHelp();}else{window.hidePopoverMenus();}});}setMobileTopbar();addSidebarItems();addSidebarCrates();onHashChange(null);window.addEventListener("hashchange",onHashChange);window.searchState.setup();}());(function(){const SIDEBAR\_MIN=100;const SIDEBAR\_MAX=500;const RUSTDOC\_MOBILE\_BREAKPOINT=700;const BODY\_MIN=400;const SIDEBAR\_VANISH\_THRESHOLD=SIDEBAR\_MIN/2;const sidebarButton=document.getElementById("sidebar-button");if(sidebarButton){sidebarButton.addEventListener("click",e=>{removeClass(document.documentElement,"hide-sidebar");updateLocalStorage("hide-sidebar","false");if(document.querySelector(".rustdoc.src")){window.rustdocToggleSrcSidebar();}e.preventDefault();});}let currentPointerId=null;let desiredSidebarSize=null;let pendingSidebarResizingFrame=false;const resizer=document.querySelector(".sidebar-resizer");const sidebar=document.querySelector(".sidebar");if(!resizer||!sidebar){return;}const isSrcPage=hasClass(document.body,"src");const hideSidebar=function(){if(isSrcPage){window.rustdocCloseSourceSidebar();updateLocalStorage("src-sidebar-width",null);document.documentElement.style.removeProperty("--src-sidebar-width");sidebar.style.removeProperty("--src-sidebar-width");resizer.style.removeProperty("--src-sidebar-width");}else{addClass(document.documentElement,"hide-sidebar");updateLocalStorage("hide-sidebar","true");updateLocalStorage("desktop-sidebar-width",null);document.documentElement.style.removeProperty("--desktop-sidebar-width");sidebar.style.removeProperty("--desktop-sidebar-width");resizer.style.removeProperty("--desktop-sidebar-width");}};const showSidebar=function(){if(isSrcPage){window.rustdocShowSourceSidebar();}else{removeClass(document.documentElement,"hide-sidebar");updateLocalStorage("hide-sidebar","false");}};const changeSidebarSize=function(size){if(isSrcPage){updateLocalStorage("src-sidebar-width",size.toString());sidebar.style.setProperty("--src-sidebar-width",size+"px");resizer.style.setProperty("--src-sidebar-width",size+"px");}else{updateLocalStorage("desktop-sidebar-width",size.toString());sidebar.style.setProperty("--desktop-sidebar-width",size+"px");resizer.style.setProperty("--desktop-sidebar-width",size+"px");}};const isSidebarHidden=function(){return isSrcPage?!hasClass(document.documentElement,"src-sidebar-expanded"):hasClass(document.documentElement,"hide-sidebar");};const resize=function(e){if(currentPointerId===null||currentPointerId!==e.pointerId){return;}e.preventDefault();const pos=e.clientX-3;if(pos<SIDEBAR\_VANISH\_THRESHOLD){hideSidebar();}else if(pos>=SIDEBAR\_MIN){if(isSidebarHidden()){showSidebar();}const constrainedPos=Math.min(pos,window.innerWidth-BODY\_MIN,SIDEBAR\_MAX);changeSidebarSize(constrainedPos);desiredSidebarSize=constrainedPos;if(pendingSidebarResizingFrame!==false){clearTimeout(pendingSidebarResizingFrame);}pendingSidebarResizingFrame=setTimeout(()=>{if(currentPointerId===null||pendingSidebarResizingFrame===false){return;}pendingSidebarResizingFrame=false;document.documentElement.style.setProperty("--resizing-sidebar-width",desiredSidebarSize+"px",);},100);}};window.addEventListener("resize",()=>{if(window.innerWidth<RUSTDOC\_MOBILE\_BREAKPOINT){return;}stopResize();if(desiredSidebarSize!==null&&desiredSidebarSize>=(window.innerWidth-BODY\_MIN)){changeSidebarSize(window.innerWidth-BODY\_MIN);}else if(desiredSidebarSize!==null&&desiredSidebarSize>SIDEBAR\_MIN){changeSidebarSize(desiredSidebarSize);}});const stopResize=function(e){if(currentPointerId===null){return;}if(e){e.preventDefault();}desiredSidebarSize=sidebar.getBoundingClientRect().width;removeClass(resizer,"active");window.removeEventListener("pointermove",resize,false);window.removeEventListener("pointerup",stopResize,false);removeClass(document.documentElement,"sidebar-resizing");document.documentElement.style.removeProperty("--resizing-sidebar-width");if(resizer.releasePointerCapture){resizer.releasePointerCapture(currentPointerId);currentPointerId=null;}};const initResize=function(e){if(currentPointerId!==null||e.altKey||e.ctrlKey||e.metaKey||e.button!==0){return;}if(resizer.setPointerCapture){resizer.setPointerCapture(e.pointerId);if(!resizer.hasPointerCapture(e.pointerId)){resizer.releasePointerCapture(e.pointerId);return;}currentPointerId=e.pointerId;}window.hideAllModals(false);e.preventDefault();window.addEventListener("pointermove",resize,false);window.addEventListener("pointercancel",stopResize,false);window.addEventListener("pointerup",stopResize,false);addClass(resizer,"active");addClass(document.documentElement,"sidebar-resizing");const pos=e.clientX-sidebar.offsetLeft-3;document.documentElement.style.setProperty("--resizing-sidebar-width",pos+"px");desiredSidebarSize=null;};resizer.addEventListener("pointerdown",initResize,false);}());(function(){function copyContentToClipboard(content){if(content===null){return;}const el=document.createElement("textarea");el.value=content;el.setAttribute("readonly","");el.style.position="absolute";el.style.left="-9999px";document.body.appendChild(el);el.select();document.execCommand("copy");document.body.removeChild(el);}function copyButtonAnimation(button){button.classList.add("clicked");if(button.reset\_button\_timeout!==undefined){clearTimeout(button.reset\_button\_timeout);}button.reset\_button\_timeout=setTimeout(()=>{button.reset\_button\_timeout=undefined;button.classList.remove("clicked");},1000);}const but=document.getElementById("copy-path");if(!but){return;}but.onclick=()=>{const titleElement=document.querySelector("title");const title=titleElement&&titleElement.textContent?titleElement.textContent.replace(" - Rust",""):"";const\[item,module\]=title.split(" in ");const path=\[item\];if(module!==undefined){path.unshift(module);}copyContentToClipboard(path.join("::"));copyButtonAnimation(but);};function copyCode(codeElem){if(!codeElem){return;}copyContentToClipboard(codeElem.textContent);}function getExampleWrap(event){const target=event.target;if(target instanceof HTMLElement){let elem=target;while(elem!==null&&!hasClass(elem,"example-wrap")){if(elem===document.body||elem.tagName==="A"||elem.tagName==="BUTTON"||hasClass(elem,"docblock")){return null;}elem=elem.parentElement;}return elem;}else{return null;}}function addCopyButton(event){const elem=getExampleWrap(event);if(elem===null){return;}elem.removeEventListener("mouseover",addCopyButton);const parent=document.createElement("div");parent.className="button-holder";const runButton=elem.querySelector(".test-arrow");if(runButton!==null){parent.appendChild(runButton);}elem.appendChild(parent);const copyButton=document.createElement("button");copyButton.className="copy-button";copyButton.title="Copy code to clipboard";copyButton.addEventListener("click",()=>{copyCode(elem.querySelector("pre > code"));copyButtonAnimation(copyButton);});parent.appendChild(copyButton);if(!elem.parentElement||!elem.parentElement.classList.contains("scraped-example")||!window.updateScrapedExample){return;}const scrapedWrapped=elem.parentElement;window.updateScrapedExample(scrapedWrapped,parent);}function showHideCodeExampleButtons(event){const elem=getExampleWrap(event);if(elem===null){return;}let buttons=elem.querySelector(".button-holder");if(buttons===null){addCopyButton(event);buttons=elem.querySelector(".button-holder");if(buttons===null){return;}}buttons.classList.toggle("keep-visible");}onEachLazy(document.querySelectorAll(".docblock .example-wrap"),elem=>{elem.addEventListener("mouseover",addCopyButton);elem.addEventListener("click",showHideCodeExampleButtons);});}());

  (function() { function applyTheme(theme) { if (theme) { document.documentElement.dataset.docsRsTheme = theme; } } window.addEventListener("storage", ev => { if (ev.key === "rustdoc-theme") { applyTheme(ev.newValue); } }); // see ./storage-change-detection.html for details window.addEventListener("message", ev => { if (ev.data && ev.data.storage && ev.data.storage.key === "rustdoc-theme") { applyTheme(ev.data.storage.value); } }); applyTheme(window.localStorage.getItem("rustdoc-theme")); })();

[Docs.rs](https://docs.rs/)

{ "name": "gix", "version": "0.72.1" }*   [gix-0.72.1](# "Interact with git repositories just like git would")
    
    *   gix 0.72.1
    *   [Permalink](https://docs.rs/gix/0.72.1/gix/struct.Repository.html "Get a link to this specific version")
    *   [Docs.rs crate page](https://docs.rs/crate/gix/latest "See gix in docs.rs")
    *   [MIT](https://spdx.org/licenses/MIT) OR [Apache-2.0](https://spdx.org/licenses/Apache-2.0)
    
    *   Links
    *   [Repository](https://github.com/GitoxideLabs/gitoxide)
    *   [crates.io](https://crates.io/crates/gix "See gix in crates.io")
    *   [Source](https://docs.rs/crate/gix/latest/source/ "Browse source of gix-0.72.1")
    
    *   Owners
    *   [Byron](https://crates.io/users/Byron)
    
    *   Dependencies
    *   *   [async-std ^1.12.0 _normal_ _optional_](https://docs.rs/async-std/^1.12.0)
        *   [document-features ^0.2.0 _normal_ _optional_](https://docs.rs/document-features/^0.2.0)
        *   [gix-actor ^0.35.1 _normal_](https://docs.rs/gix-actor/^0.35.1)
        *   [gix-archive ^0.21.1 _normal_ _optional_](https://docs.rs/gix-archive/^0.21.1)
        *   [gix-attributes ^0.26.0 _normal_ _optional_](https://docs.rs/gix-attributes/^0.26.0)
        *   [gix-blame ^0.2.1 _normal_ _optional_](https://docs.rs/gix-blame/^0.2.1)
        *   [gix-command ^0.6.0 _normal_ _optional_](https://docs.rs/gix-command/^0.6.0)
        *   [gix-commitgraph ^0.28.0 _normal_](https://docs.rs/gix-commitgraph/^0.28.0)
        *   [gix-config ^0.45.1 _normal_](https://docs.rs/gix-config/^0.45.1)
        *   [gix-credentials ^0.29.0 _normal_ _optional_](https://docs.rs/gix-credentials/^0.29.0)
        *   [gix-date ^0.10.1 _normal_](https://docs.rs/gix-date/^0.10.1)
        *   [gix-diff ^0.52.1 _normal_](https://docs.rs/gix-diff/^0.52.1)
        *   [gix-dir ^0.14.1 _normal_ _optional_](https://docs.rs/gix-dir/^0.14.1)
        *   [gix-discover ^0.40.1 _normal_](https://docs.rs/gix-discover/^0.40.1)
        *   [gix-features ^0.42.1 _normal_](https://docs.rs/gix-features/^0.42.1)
        *   [gix-filter ^0.19.1 _normal_ _optional_](https://docs.rs/gix-filter/^0.19.1)
        *   [gix-fs ^0.15.0 _normal_](https://docs.rs/gix-fs/^0.15.0)
        *   [gix-glob ^0.20.0 _normal_](https://docs.rs/gix-glob/^0.20.0)
        *   [gix-hash ^0.18.0 _normal_](https://docs.rs/gix-hash/^0.18.0)
        *   [gix-hashtable ^0.8.1 _normal_](https://docs.rs/gix-hashtable/^0.8.1)
        *   [gix-ignore ^0.15.0 _normal_ _optional_](https://docs.rs/gix-ignore/^0.15.0)
        *   [gix-index ^0.40.0 _normal_ _optional_](https://docs.rs/gix-index/^0.40.0)
        *   [gix-lock ^17.1.0 _normal_](https://docs.rs/gix-lock/^17.1.0)
        *   [gix-mailmap ^0.27.1 _normal_ _optional_](https://docs.rs/gix-mailmap/^0.27.1)
        *   [gix-merge ^0.5.1 _normal_ _optional_](https://docs.rs/gix-merge/^0.5.1)
        *   [gix-negotiate ^0.20.1 _normal_ _optional_](https://docs.rs/gix-negotiate/^0.20.1)
        *   [gix-object ^0.49.1 _normal_](https://docs.rs/gix-object/^0.49.1)
        *   [gix-odb ^0.69.1 _normal_](https://docs.rs/gix-odb/^0.69.1)
        *   [gix-pack ^0.59.1 _normal_](https://docs.rs/gix-pack/^0.59.1)
        *   [gix-path ^0.10.17 _normal_](https://docs.rs/gix-path/^0.10.17)
        *   [gix-pathspec ^0.11.0 _normal_ _optional_](https://docs.rs/gix-pathspec/^0.11.0)
        *   [gix-prompt ^0.11.0 _normal_ _optional_](https://docs.rs/gix-prompt/^0.11.0)
        *   [gix-protocol ^0.50.1 _normal_](https://docs.rs/gix-protocol/^0.50.1)
        *   [gix-ref ^0.52.1 _normal_](https://docs.rs/gix-ref/^0.52.1)
        *   [gix-refspec ^0.30.1 _normal_](https://docs.rs/gix-refspec/^0.30.1)
        *   [gix-revision ^0.34.1 _normal_](https://docs.rs/gix-revision/^0.34.1)
        *   [gix-revwalk ^0.20.1 _normal_](https://docs.rs/gix-revwalk/^0.20.1)
        *   [gix-sec ^0.11.0 _normal_](https://docs.rs/gix-sec/^0.11.0)
        *   [gix-shallow ^0.4.0 _normal_](https://docs.rs/gix-shallow/^0.4.0)
        *   [gix-status ^0.19.1 _normal_ _optional_](https://docs.rs/gix-status/^0.19.1)
        *   [gix-submodule ^0.19.1 _normal_ _optional_](https://docs.rs/gix-submodule/^0.19.1)
        *   [gix-tempfile ^17.1.0 _normal_](https://docs.rs/gix-tempfile/^17.1.0)
        *   [gix-trace ^0.1.12 _normal_](https://docs.rs/gix-trace/^0.1.12)
        *   [gix-transport ^0.47.0 _normal_ _optional_](https://docs.rs/gix-transport/^0.47.0)
        *   [gix-traverse ^0.46.1 _normal_](https://docs.rs/gix-traverse/^0.46.1)
        *   [gix-url ^0.31.0 _normal_](https://docs.rs/gix-url/^0.31.0)
        *   [gix-utils ^0.3.0 _normal_](https://docs.rs/gix-utils/^0.3.0)
        *   [gix-validate ^0.10.0 _normal_](https://docs.rs/gix-validate/^0.10.0)
        *   [gix-worktree ^0.41.0 _normal_ _optional_](https://docs.rs/gix-worktree/^0.41.0)
        *   [gix-worktree-state ^0.19.0 _normal_ _optional_](https://docs.rs/gix-worktree-state/^0.19.0)
        *   [gix-worktree-stream ^0.21.1 _normal_ _optional_](https://docs.rs/gix-worktree-stream/^0.21.1)
        *   [once\_cell ^1.21.3 _normal_](https://docs.rs/once_cell/^1.21.3)
        *   [parking\_lot ^0.12.1 _normal_ _optional_](https://docs.rs/parking_lot/^0.12.1)
        *   [prodash ^29.0.2 _normal_ _optional_](https://docs.rs/prodash/^29.0.2)
        *   [regex ^1.6.0 _normal_ _optional_](https://docs.rs/regex/^1.6.0)
        *   [serde ^1.0.114 _normal_ _optional_](https://docs.rs/serde/^1.0.114)
        *   [signal-hook ^0.3.9 _normal_ _optional_](https://docs.rs/signal-hook/^0.3.9)
        *   [smallvec ^1.15.0 _normal_](https://docs.rs/smallvec/^1.15.0)
        *   [thiserror ^2.0.0 _normal_](https://docs.rs/thiserror/^2.0.0)
        *   [anyhow ^1 _dev_](https://docs.rs/anyhow/^1)
        *   [async-std ^1.12.0 _dev_](https://docs.rs/async-std/^1.12.0)
        *   [insta ^1.40.0 _dev_](https://docs.rs/insta/^1.40.0)
        *   [is\_ci ^1.1.1 _dev_](https://docs.rs/is_ci/^1.1.1)
        *   [pretty\_assertions ^1.4.0 _dev_](https://docs.rs/pretty_assertions/^1.4.0)
        *   [serial\_test ^3.1.0 _dev_](https://docs.rs/serial_test/^3.1.0)
        *   [termtree ^0.5.1 _dev_](https://docs.rs/termtree/^0.5.1)
        *   [walkdir ^2.3.2 _dev_](https://docs.rs/walkdir/^2.3.2)
        
    
    *   Versions
    
    *   [**100%** of the crate is documented](https://docs.rs/crate/gix/latest)
    
*   [Platform](#)
    *   [x86\_64-unknown-linux-gnu](https://docs.rs/crate/gix/latest/target-redirect/x86_64-unknown-linux-gnu/gix/struct.Repository.html)
*   [Feature flags](https://docs.rs/crate/gix/latest/features "Browse available feature flags of gix-0.72.1")

*   [docs.rs](#)
    *   [About docs.rs](https://docs.rs/about)
    *   [Privacy policy](https://foundation.rust-lang.org/policies/privacy-policy/#docs.rs)

*   [Rust](#)
    *   [Rust website](https://www.rust-lang.org/)
    *   [The Book](https://doc.rust-lang.org/book/)
    *   [Standard Library API Reference](https://doc.rust-lang.org/std/)
    *   [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
    *   [The Cargo Guide](https://doc.rust-lang.org/cargo/guide/)
    *   [Clippy Documentation](https://doc.rust-lang.org/nightly/clippy)

// Allow menus to be opened and used by keyboard. (function() { const updateMenuPositionForSubMenu = currentMenuSupplier => { const currentMenu = currentMenuSupplier(); const subMenu = currentMenu?.getElementsByClassName("pure-menu-children")?.\[0\]; subMenu?.style.setProperty("--menu-x", \`${currentMenu.getBoundingClientRect().x}px\`); }; const loadedMenus = new Set(); async function loadAjaxMenu(menu, id, msg) { if (loadedMenus.has(id)) { return; } loadedMenus.add(id); if (!menu.querySelector(".rotate")) { return; } const listElem = document.getElementById(id); if (!listElem) { // We're not in a documentation page, so no need to do anything. return; } const url = listElem.dataset.url; try { const response = await fetch(url); listElem.innerHTML = await response.text(); } catch (ex) { console.error(\`Failed to load ${msg}: ${ex}\`); listElem.innerHTML = \`Failed to load ${msg}\`; } } let currentMenu; const backdrop = document.createElement("div"); backdrop.style = "display:none;position:fixed;width:100%;height:100%;z-index:1"; document.documentElement.insertBefore(backdrop, document.querySelector("body")); addEventListener("resize", () => updateMenuPositionForSubMenu(() => currentMenu)); function previous(allItems, item) { let i = 1; const l = allItems.length; while (i < l) { if (allItems\[i\] === item) { return allItems\[i - 1\]; } i += 1; } } function next(allItems, item) { let i = 0; const l = allItems.length - 1; while (i < l) { if (allItems\[i\] === item) { return allItems\[i + 1\]; } i += 1; } } function last(allItems) { return allItems\[allItems.length - 1\]; } function closeMenu() { if (this === backdrop) { document.documentElement.focus(); } else if (currentMenu.querySelector(".pure-menu-link:focus")) { currentMenu.firstElementChild.focus(); } currentMenu.classList.remove("pure-menu-active"); currentMenu = null; backdrop.style.display = "none"; } backdrop.onclick = closeMenu; function openMenu(newMenu) { updateMenuPositionForSubMenu(() => newMenu); currentMenu = newMenu; newMenu.classList.add("pure-menu-active"); backdrop.style.display = "block"; if (newMenu.querySelector("#releases-list")) { loadAjaxMenu( newMenu, "releases-list", "release list", ); } else if (newMenu.querySelector("#platforms")) { loadAjaxMenu( newMenu, "platforms", "platforms list", ); } } function menuOnClick(e) { if (this.getAttribute("href") !== "#") { return; } if (this.parentNode === currentMenu) { closeMenu(); this.blur(); } else { if (currentMenu) closeMenu(); openMenu(this.parentNode); } e.preventDefault(); e.stopPropagation(); } function menuKeyDown(e) { if (currentMenu) { const children = currentMenu.querySelector(".pure-menu-children"); const currentLink = children.querySelector(".pure-menu-link:focus"); let currentItem; if (currentLink && currentLink.parentNode.classList.contains("pure-menu-item")) { currentItem = currentLink.parentNode; } let allItems = \[\]; if (children) { allItems = children.querySelectorAll(".pure-menu-item .pure-menu-link"); } let switchTo = null; switch (e.key.toLowerCase()) { case "escape": case "esc": closeMenu(); e.preventDefault(); e.stopPropagation(); return; case "arrowdown": case "down": if (currentLink) { // Arrow down when an item other than the last is focused: // focus next item. // Arrow down when the last item is focused: // jump to top. switchTo = (next(allItems, currentLink) || allItems\[0\]); } else { // Arrow down when a menu is open and nothing is focused: // focus first item. switchTo = allItems\[0\]; } break; case "arrowup": case "up": if (currentLink) { // Arrow up when an item other than the first is focused: // focus previous item. // Arrow up when the first item is focused: // jump to bottom. switchTo = (previous(allItems, currentLink) || last(allItems)); } else { // Arrow up when a menu is open and nothing is focused: focus last item. switchTo = last(allItems); } break; case "tab": if (!currentLink) { // if the menu is open, we should focus trap into it // // this is the behavior of the WAI example, it is not the same as GitHub, // but GitHub allows you to tab yourself out of the menu without closing it // (which is horrible behavior) switchTo = e.shiftKey ? last(allItems) : allItems\[0\]; } else if (e.shiftKey && currentLink === allItems\[0\]) { // if you tab your way out of the menu, close it // // this is neither what GitHub nor the WAI example do, but is a // rationalization of GitHub's behavior: we don't want users who know how to // use tab and enter, but don't know that they can close menus with Escape, // to find themselves completely trapped in the menu closeMenu(); e.preventDefault(); e.stopPropagation(); } else if (!e.shiftKey && currentLink === last(allItems)) { // same as above. // if you tab your way out of the menu, close it closeMenu(); } break; case "enter": case "return": // enter and return have the default browser behavior, // but they also close the menu // this behavior is identical between both the WAI example, and GitHub's setTimeout(() => { closeMenu(); }, 100); break; case "space": case " ": { // space closes the menu, and activates the current link // this behavior is identical between both the WAI example, and GitHub's const hasPopup = document.activeElement instanceof HTMLAnchorElement && !document.activeElement.hasAttribute("aria-haspopup"); if (hasPopup) { // It's supposed to copy the behaviour of the WAI Menu Bar // page, and of GitHub's menus. I've been using these two // sources to judge what is basically "industry standard" // behaviour for menu keyboard activity on the web. // // On GitHub, here's what I notice: // // 1 If you click open a menu, the menu button remains // focused. If, in this stage, I press space, the menu will // close. // // 2 If I use the arrow keys to focus a menu item, and then // press space, the menu item will be activated. For // example, clicking "+", then pressing down, then pressing // space will open the New Repository page. // // Behaviour 1 is why the // \`!document.activeElement.hasAttribute("aria-haspopup")\` // condition is there. It's to make sure the menu-link on // things like the About dropdown don't get activated. // Behaviour 2 is why this code is required at all; I want to // activate the currently highlighted menu item. document.activeElement.click(); } setTimeout(() => { closeMenu(); }, 100); e.preventDefault(); e.stopPropagation(); break; } case "home": // home: focus first menu item. // // This is the behavior of WAI, while GitHub scrolls, but it's unlikely that a // user will try to scroll the page while the menu is open, so they won't do it // on accident. switchTo = allItems\[0\]; break; case "end": // end: focus last menu item. // // This is the behavior of WAI, while GitHub scrolls, but it's unlikely that a // user will try to scroll the page while the menu is open, so they won't do it // on accident. switchTo = last(allItems); break; case "pageup": // page up: jump five items up, stopping at the top // // the number 5 is used so that we go one page in the inner-scrolled // Dependencies and Versions fields switchTo = currentItem || allItems\[0\]; for (let n = 0; n < 5; ++n) { const hasPrevious = switchTo.previousElementSibling && switchTo.previousElementSibling.classList.contains("pure-menu-item"); if (hasPrevious) { switchTo = switchTo.previousElementSibling; } } break; case "pagedown": // page down: jump five items down, stopping at the bottom // the number 5 is used so that we go one page in the // inner-scrolled Dependencies and Versions fields switchTo = currentItem || last(allItems); for (let n = 0; n < 5; ++n) { const hasNext = switchTo.nextElementSibling && switchTo.nextElementSibling.classList.contains("pure-menu-item"); if (hasNext) { switchTo = switchTo.nextElementSibling; } } break; } if (switchTo) { const switchToLink = switchTo.querySelector("a"); if (switchToLink) { switchToLink.focus(); } else { switchTo.focus(); } e.preventDefault(); e.stopPropagation(); } } else if (e.target.parentNode && e.target.parentNode.classList && e.target.parentNode.classList.contains("pure-menu-has-children") ) { switch (e.key.toLowerCase()) { case "arrowdown": case "down": case "space": case " ": openMenu(e.target.parentNode); e.preventDefault(); e.stopPropagation(); break; } } } for (const menu of document.querySelectorAll(".pure-menu-has-children")) { menu.firstElementChild.setAttribute("aria-haspopup", "menu"); menu.firstElementChild.nextElementSibling.setAttribute("role", "menu"); menu.firstElementChild.addEventListener("click", menuOnClick); } document.documentElement.addEventListener("keydown", menuKeyDown); document.documentElement.addEventListener("keydown", ev => { if (ev.key === "y" && ev.target.tagName !== "INPUT") { const permalink = document.getElementById("permalink"); if (document.location.hash !== "") { permalink.href += document.location.hash; } history.replaceState({}, null, permalink.href); } }); })(); (function() { const clipboard = document.getElementById("clipboard"); if (clipboard) { let resetClipboardTimeout = null; const resetClipboardIcon = clipboard.innerHTML; clipboard.addEventListener("click", () => { const metadata = JSON.parse(document.getElementById("crate-metadata").innerText); const temporaryInput = document.createElement("input"); temporaryInput.type = "text"; temporaryInput.value = \`${metadata.name} = "${metadata.version}"\`; document.body.append(temporaryInput); temporaryInput.select(); document.execCommand("copy"); temporaryInput.remove(); clipboard.textContent = "✓"; if (resetClipboardTimeout !== null) { clearTimeout(resetClipboardTimeout); } resetClipboardTimeout = setTimeout(() => { resetClipboardTimeout = null; clipboard.innerHTML = resetClipboardIcon; }, 1000); }); } for (const e of document.querySelectorAll("a\[data-fragment=\\"retain\\"\]")) { e.addEventListener("mouseover", () => { e.hash = document.location.hash; }); } })();

[gix](https://docs.rs/gix/latest/gix/index.html)0.72.1
------------------------------------------------------

[Repository](#)
---------------

### [Sections](#)

*   [`Send` only with `parallel` feature](#send-only-with-parallel-feature "`Send` only with `parallel` feature")

### [Fields](#fields)

*   [objects](#structfield.objects "objects")
*   [refs](#structfield.refs "refs")

### [Methods](#implementations)

*   [attributes](#method.attributes "attributes")
*   [attributes\_only](#method.attributes_only "attributes_only")
*   [author](#method.author "author")
*   [big\_file\_threshold](#method.big_file_threshold "big_file_threshold")
*   [blob\_merge\_options](#method.blob_merge_options "blob_merge_options")
*   [branch\_names](#method.branch_names "branch_names")
*   [branch\_remote](#method.branch_remote "branch_remote")
*   [branch\_remote\_name](#method.branch_remote_name "branch_remote_name")
*   [branch\_remote\_ref\_name](#method.branch_remote_ref_name "branch_remote_ref_name")
*   [branch\_remote\_tracking\_ref\_name](#method.branch_remote_tracking_ref_name "branch_remote_tracking_ref_name")
*   [checkout\_options](#method.checkout_options "checkout_options")
*   [clear\_namespace](#method.clear_namespace "clear_namespace")
*   [command\_context](#method.command_context "command_context")
*   [commit](#method.commit "commit")
*   [commit\_as](#method.commit_as "commit_as")
*   [commit\_graph](#method.commit_graph "commit_graph")
*   [commit\_graph\_if\_enabled](#method.commit_graph_if_enabled "commit_graph_if_enabled")
*   [committer](#method.committer "committer")
*   [common\_dir](#method.common_dir "common_dir")
*   [compute\_object\_cache\_size\_for\_tree\_diffs](#method.compute_object_cache_size_for_tree_diffs "compute_object_cache_size_for_tree_diffs")
*   [config\_snapshot](#method.config_snapshot "config_snapshot")
*   [config\_snapshot\_mut](#method.config_snapshot_mut "config_snapshot_mut")
*   [current\_dir](#method.current_dir "current_dir")
*   [diff\_algorithm](#method.diff_algorithm "diff_algorithm")
*   [diff\_resource\_cache](#method.diff_resource_cache "diff_resource_cache")
*   [diff\_resource\_cache\_for\_tree\_diff](#method.diff_resource_cache_for_tree_diff "diff_resource_cache_for_tree_diff")
*   [diff\_tree\_to\_tree](#method.diff_tree_to_tree "diff_tree_to_tree")
*   [dirwalk](#method.dirwalk "dirwalk")
*   [dirwalk\_iter](#method.dirwalk_iter "dirwalk_iter")
*   [dirwalk\_options](#method.dirwalk_options "dirwalk_options")
*   [edit\_reference](#method.edit_reference "edit_reference")
*   [edit\_references](#method.edit_references "edit_references")
*   [edit\_references\_as](#method.edit_references_as "edit_references_as")
*   [edit\_tree](#method.edit_tree "edit_tree")
*   [empty\_blob](#method.empty_blob "empty_blob")
*   [empty\_reusable\_buffer](#method.empty_reusable_buffer "empty_reusable_buffer")
*   [empty\_tree](#method.empty_tree "empty_tree")
*   [excludes](#method.excludes "excludes")
*   [filesystem\_options](#method.filesystem_options "filesystem_options")
*   [filter\_pipeline](#method.filter_pipeline "filter_pipeline")
*   [find\_blob](#method.find_blob "find_blob")
*   [find\_commit](#method.find_commit "find_commit")
*   [find\_default\_remote](#method.find_default_remote "find_default_remote")
*   [find\_fetch\_remote](#method.find_fetch_remote "find_fetch_remote")
*   [find\_header](#method.find_header "find_header")
*   [find\_object](#method.find_object "find_object")
*   [find\_reference](#method.find_reference "find_reference")
*   [find\_remote](#method.find_remote "find_remote")
*   [find\_tag](#method.find_tag "find_tag")
*   [find\_tree](#method.find_tree "find_tree")
*   [git\_dir](#method.git_dir "git_dir")
*   [git\_dir\_trust](#method.git_dir_trust "git_dir_trust")
*   [has\_object](#method.has_object "has_object")
*   [head](#method.head "head")
*   [head\_commit](#method.head_commit "head_commit")
*   [head\_id](#method.head_id "head_id")
*   [head\_name](#method.head_name "head_name")
*   [head\_ref](#method.head_ref "head_ref")
*   [head\_tree](#method.head_tree "head_tree")
*   [head\_tree\_id](#method.head_tree_id "head_tree_id")
*   [head\_tree\_id\_or\_empty](#method.head_tree_id_or_empty "head_tree_id_or_empty")
*   [index](#method.index "index")
*   [index\_from\_tree](#method.index_from_tree "index_from_tree")
*   [index\_or\_empty](#method.index_or_empty "index_or_empty")
*   [index\_or\_load\_from\_head](#method.index_or_load_from_head "index_or_load_from_head")
*   [index\_or\_load\_from\_head\_or\_empty](#method.index_or_load_from_head_or_empty "index_or_load_from_head_or_empty")
*   [index\_path](#method.index_path "index_path")
*   [index\_worktree\_status](#method.index_worktree_status "index_worktree_status")
*   [install\_dir](#method.install_dir "install_dir")
*   [into\_sync](#method.into_sync "into_sync")
*   [is\_bare](#method.is_bare "is_bare")
*   [is\_dirty](#method.is_dirty "is_dirty")
*   [is\_shallow](#method.is_shallow "is_shallow")
*   [kind](#method.kind "kind")
*   [main\_repo](#method.main_repo "main_repo")
*   [merge\_base](#method.merge_base "merge_base")
*   [merge\_base\_octopus](#method.merge_base_octopus "merge_base_octopus")
*   [merge\_base\_octopus\_with\_graph](#method.merge_base_octopus_with_graph "merge_base_octopus_with_graph")
*   [merge\_base\_with\_graph](#method.merge_base_with_graph "merge_base_with_graph")
*   [merge\_bases\_many\_with\_graph](#method.merge_bases_many_with_graph "merge_bases_many_with_graph")
*   [merge\_commits](#method.merge_commits "merge_commits")
*   [merge\_resource\_cache](#method.merge_resource_cache "merge_resource_cache")
*   [merge\_trees](#method.merge_trees "merge_trees")
*   [modules](#method.modules "modules")
*   [modules\_path](#method.modules_path "modules_path")
*   [namespace](#method.namespace "namespace")
*   [object\_cache\_size](#method.object_cache_size "object_cache_size")
*   [object\_cache\_size\_if\_unset](#method.object_cache_size_if_unset "object_cache_size_if_unset")
*   [object\_hash](#method.object_hash "object_hash")
*   [open\_index](#method.open_index "open_index")
*   [open\_mailmap](#method.open_mailmap "open_mailmap")
*   [open\_mailmap\_into](#method.open_mailmap_into "open_mailmap_into")
*   [open\_modules\_file](#method.open_modules_file "open_modules_file")
*   [open\_options](#method.open_options "open_options")
*   [path](#method.path "path")
*   [pathspec](#method.pathspec "pathspec")
*   [pathspec\_defaults](#method.pathspec_defaults "pathspec_defaults")
*   [pathspec\_defaults\_inherit\_ignore\_case](#method.pathspec_defaults_inherit_ignore_case "pathspec_defaults_inherit_ignore_case")
*   [prefix](#method.prefix "prefix")
*   [reference](#method.reference "reference")
*   [references](#method.references "references")
*   [remote\_at](#method.remote_at "remote_at")
*   [remote\_at\_without\_url\_rewrite](#method.remote_at_without_url_rewrite "remote_at_without_url_rewrite")
*   [remote\_default\_name](#method.remote_default_name "remote_default_name")
*   [remote\_names](#method.remote_names "remote_names")
*   [rev\_parse](#method.rev_parse "rev_parse")
*   [rev\_parse\_single](#method.rev_parse_single "rev_parse_single")
*   [rev\_walk](#method.rev_walk "rev_walk")
*   [revision\_graph](#method.revision_graph "revision_graph")
*   [set\_freelist](#method.set_freelist "set_freelist")
*   [set\_namespace](#method.set_namespace "set_namespace")
*   [shallow\_commits](#method.shallow_commits "shallow_commits")
*   [shallow\_file](#method.shallow_file "shallow_file")
*   [ssh\_connect\_options](#method.ssh_connect_options "ssh_connect_options")
*   [stat\_options](#method.stat_options "stat_options")
*   [state](#method.state "state")
*   [status](#method.status "status")
*   [submodules](#method.submodules "submodules")
*   [tag](#method.tag "tag")
*   [tag\_reference](#method.tag_reference "tag_reference")
*   [transport\_options](#method.transport_options "transport_options")
*   [tree\_index\_status](#method.tree_index_status "tree_index_status")
*   [tree\_merge\_options](#method.tree_merge_options "tree_merge_options")
*   [try\_find\_header](#method.try_find_header "try_find_header")
*   [try\_find\_object](#method.try_find_object "try_find_object")
*   [try\_find\_reference](#method.try_find_reference "try_find_reference")
*   [try\_find\_remote](#method.try_find_remote "try_find_remote")
*   [try\_find\_remote\_without\_url\_rewrite](#method.try_find_remote_without_url_rewrite "try_find_remote_without_url_rewrite")
*   [try\_index](#method.try_index "try_index")
*   [upstream\_branch\_and\_remote\_for\_tracking\_branch](#method.upstream_branch_and_remote_for_tracking_branch "upstream_branch_and_remote_for_tracking_branch")
*   [virtual\_merge\_base](#method.virtual_merge_base "virtual_merge_base")
*   [virtual\_merge\_base\_with\_graph](#method.virtual_merge_base_with_graph "virtual_merge_base_with_graph")
*   [with\_object\_memory](#method.with_object_memory "with_object_memory")
*   [without\_freelist](#method.without_freelist "without_freelist")
*   [work\_dir](#method.work_dir "work_dir")
*   [workdir](#method.workdir "workdir")
*   [workdir\_path](#method.workdir_path "workdir_path")
*   [worktree](#method.worktree "worktree")
*   [worktree\_archive](#method.worktree_archive "worktree_archive")
*   [worktree\_stream](#method.worktree_stream "worktree_stream")
*   [worktrees](#method.worktrees "worktrees")
*   [write\_blob](#method.write_blob "write_blob")
*   [write\_blob\_stream](#method.write_blob_stream "write_blob_stream")
*   [write\_object](#method.write_object "write_object")

### [Trait Implementations](#trait-implementations)

*   [Clone](#impl-Clone-for-Repository "Clone")
*   [Debug](#impl-Debug-for-Repository "Debug")
*   [Exists](#impl-Exists-for-Repository "Exists")
*   [Find](#impl-Find-for-Repository "Find")
*   [From<&ThreadSafeRepository>](#impl-From%3C%26ThreadSafeRepository%3E-for-Repository "From<&ThreadSafeRepository>")
*   [From<PrepareCheckout>](#impl-From%3CPrepareCheckout%3E-for-Repository "From<PrepareCheckout>")
*   [From<PrepareFetch>](#impl-From%3CPrepareFetch%3E-for-Repository "From<PrepareFetch>")
*   [From<Repository>](#impl-From%3CRepository%3E-for-ThreadSafeRepository "From<Repository>")
*   [From<ThreadSafeRepository>](#impl-From%3CThreadSafeRepository%3E-for-Repository "From<ThreadSafeRepository>")
*   [Header](#impl-FindHeader-for-Repository "Header")
*   [PartialEq](#impl-PartialEq-for-Repository "PartialEq")
*   [Write](#impl-Write-for-Repository "Write")

### [Auto Trait Implementations](#synthetic-implementations)

*   [!Freeze](#impl-Freeze-for-Repository "!Freeze")
*   [!RefUnwindSafe](#impl-RefUnwindSafe-for-Repository "!RefUnwindSafe")
*   [!Sync](#impl-Sync-for-Repository "!Sync")
*   [!UnwindSafe](#impl-UnwindSafe-for-Repository "!UnwindSafe")
*   [Send](#impl-Send-for-Repository "Send")
*   [Unpin](#impl-Unpin-for-Repository "Unpin")

### [Blanket Implementations](#blanket-implementations)

*   [Any](#impl-Any-for-T "Any")
*   [Borrow<T>](#impl-Borrow%3CT%3E-for-T "Borrow<T>")
*   [BorrowMut<T>](#impl-BorrowMut%3CT%3E-for-T "BorrowMut<T>")
*   [CloneToUninit](#impl-CloneToUninit-for-T "CloneToUninit")
*   [ErasedDestructor](#impl-ErasedDestructor-for-T "ErasedDestructor")
*   [FindExt](#impl-FindExt-for-T "FindExt")
*   [FindObjectOrHeader](#impl-FindObjectOrHeader-for-T "FindObjectOrHeader")
*   [From<T>](#impl-From%3CT%3E-for-T "From<T>")
*   [Into<U>](#impl-Into%3CU%3E-for-T "Into<U>")
*   [MaybeSendSync](#impl-MaybeSendSync-for-T "MaybeSendSync")
*   [Same](#impl-Same-for-T "Same")
*   [ToOwned](#impl-ToOwned-for-T "ToOwned")
*   [TryFrom<U>](#impl-TryFrom%3CU%3E-for-T "TryFrom<U>")
*   [TryInto<U>](#impl-TryInto%3CU%3E-for-T "TryInto<U>")

[In crate gix](https://docs.rs/gix/latest/gix/index.html)
---------------------------------------------------------

[gix](https://docs.rs/gix/latest/gix/index.html)

Struct RepositoryCopy item path
===============================

[Source](https://docs.rs/gix/latest/src/gix/types.rs.html#157-179)

    pub struct Repository {
        pub refs: RefStore,
        pub objects: OdbHandle,
        /* private fields */
    }

Expand description

A thread-local handle to interact with a repository from a single thread.

It is `Send` but **not** `Sync` - for the latter you can convert it `to_sync()`. Note that it clones itself so that it is empty, requiring the user to configure each clone separately, specifically and explicitly. This is to have the fastest-possible default configuration available by default, but allow those who experiment with workloads to get speed boosts of 2x or more.

#### [§](#send-only-with-parallel-feature)`Send` only with `parallel` feature

When built with `default-features = false`, this type is **not** `Send`. The minimal feature set to activate `Send` is `features = ["parallel"]`.

Fields[§](#fields)
------------------

[§](#structfield.refs)`refs: [RefStore](https://docs.rs/gix/latest/gix/type.RefStore.html "type gix::RefStore")`

A ref store with shared ownership (or the equivalent of it).

[§](#structfield.objects)`objects: [OdbHandle](https://docs.rs/gix/latest/gix/type.OdbHandle.html "type gix::OdbHandle")`

A way to access objects.

Implementations[§](#implementations)
------------------------------------

[Source](https://docs.rs/gix/latest/src/gix/repository/attributes.rs.html#14-139)[§](#impl-Repository)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/attributes.rs.html#28-61)

#### pub fn [attributes](#method.attributes)( &self, index: &[State](https://docs.rs/gix/latest/gix/index/struct.State.html "struct gix::index::State"), attributes\_source: [Source](https://docs.rs/gix/latest/gix/worktree/stack/state/attributes/enum.Source.html "enum gix::worktree::stack::state::attributes::Source"), ignore\_source: [Source](https://docs.rs/gix/latest/gix/worktree/stack/state/ignore/enum.Source.html "enum gix::worktree::stack::state::ignore::Source"), exclude\_overrides: [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Search](https://docs.rs/gix/latest/gix/worktree/ignore/struct.Search.html "struct gix::worktree::ignore::Search")\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[AttributeStack](https://docs.rs/gix/latest/gix/struct.AttributeStack.html "struct gix::AttributeStack")<'\_>, [Error](https://docs.rs/gix/latest/gix/repository/attributes/enum.Error.html "enum gix::repository::attributes::Error")\>

Available on **(crate features `attributes` or `excludes`) and crate feature `attributes`** only.

Configure a file-system cache for accessing git attributes _and_ excludes on a per-path basis.

Use `attribute_source` to specify where to read attributes from. Also note that exclude information will always try to read `.gitignore` files from disk before trying to read it from the `index`.

Note that no worktree is required for this to work, even though access to in-tree `.gitattributes` and `.gitignore` files would require a non-empty `index` that represents a git tree.

This takes into consideration all the usual repository configuration, namely:

*   `$XDG_CONFIG_HOME/…/ignore|attributes` if `core.excludesFile|attributesFile` is _not_ set, otherwise use the configured file.
*   `$GIT_DIR/info/exclude|attributes` if present.

[Source](https://docs.rs/gix/latest/src/gix/repository/attributes.rs.html#65-93)

#### pub fn [attributes\_only](#method.attributes_only)( &self, index: &[State](https://docs.rs/gix/latest/gix/index/struct.State.html "struct gix::index::State"), attributes\_source: [Source](https://docs.rs/gix/latest/gix/worktree/stack/state/attributes/enum.Source.html "enum gix::worktree::stack::state::attributes::Source"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[AttributeStack](https://docs.rs/gix/latest/gix/struct.AttributeStack.html "struct gix::AttributeStack")<'\_>, [Error](https://docs.rs/gix/latest/gix/config/attribute_stack/enum.Error.html "enum gix::config::attribute_stack::Error")\>

Available on **(crate features `attributes` or `excludes`) and crate feature `attributes`** only.

Like [attributes()](https://docs.rs/gix/latest/gix/struct.Repository.html#method.attributes "method gix::Repository::attributes"), but without access to exclude/ignore information.

[Source](https://docs.rs/gix/latest/src/gix/repository/attributes.rs.html#110-138)

#### pub fn [excludes](#method.excludes)( &self, index: &[State](https://docs.rs/gix/latest/gix/index/struct.State.html "struct gix::index::State"), overrides: [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Search](https://docs.rs/gix/latest/gix/worktree/ignore/struct.Search.html "struct gix::worktree::ignore::Search")\>, source: [Source](https://docs.rs/gix/latest/gix/worktree/stack/state/ignore/enum.Source.html "enum gix::worktree::stack::state::ignore::Source"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[AttributeStack](https://docs.rs/gix/latest/gix/struct.AttributeStack.html "struct gix::AttributeStack")<'\_>, [Error](https://docs.rs/gix/latest/gix/config/exclude_stack/enum.Error.html "enum gix::config::exclude_stack::Error")\>

Available on **(crate features `attributes` or `excludes`) and crate feature `excludes`** only.

Configure a file-system cache checking if files below the repository are excluded, reading `.gitignore` files from the specified `source`.

Note that no worktree is required for this to work, even though access to in-tree `.gitignore` files would require a non-empty `index` that represents a tree with `.gitignore` files.

This takes into consideration all the usual repository configuration, namely:

*   `$XDG_CONFIG_HOME/…/ignore` if `core.excludesFile` is _not_ set, otherwise use the configured file.
*   `$GIT_DIR/info/exclude` if present.

When only excludes are desired, this is the most efficient way to obtain them. Otherwise use [`Repository::attributes()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.attributes "method gix::Repository::attributes") for accessing both attributes and excludes.

[Source](https://docs.rs/gix/latest/src/gix/repository/cache.rs.html#2-41)[§](#impl-Repository-1)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Configure how caches are used to speed up various git repository operations

[Source](https://docs.rs/gix/latest/src/gix/repository/cache.rs.html#11-20)

#### pub fn [object\_cache\_size](#method.object_cache_size)(&mut self, bytes: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[usize](https://doc.rust-lang.org/nightly/std/primitive.usize.html)\>>)

Sets the amount of space used at most for caching most recently accessed fully decoded objects, to `Some(bytes)`, or `None` to deactivate it entirely.

Note that it is unset by default but can be enabled once there is time for performance optimization. Well-chosen cache sizes can improve performance particularly if objects are accessed multiple times in a row. The cache is configured to grow gradually.

Note that a cache on application level should be considered as well as the best object access is not doing one.

[Source](https://docs.rs/gix/latest/src/gix/repository/cache.rs.html#25-29)

#### pub fn [object\_cache\_size\_if\_unset](#method.object_cache_size_if_unset)(&mut self, bytes: [usize](https://doc.rust-lang.org/nightly/std/primitive.usize.html))

Set an object cache of size `bytes` if none is set.

Use this method to avoid overwriting any existing value while assuring better performance in case no value is set.

[Source](https://docs.rs/gix/latest/src/gix/repository/cache.rs.html#36-40)

#### pub fn [compute\_object\_cache\_size\_for\_tree\_diffs](#method.compute_object_cache_size_for_tree_diffs)(&self, index: &[State](https://docs.rs/gix/latest/gix/index/struct.State.html "struct gix::index::State")) -> [usize](https://doc.rust-lang.org/nightly/std/primitive.usize.html)

Available on **crate feature `index`** only.

Return the amount of bytes the object cache [should be set to](https://docs.rs/gix/latest/gix/struct.Repository.html#method.object_cache_size_if_unset "method gix::Repository::object_cache_size_if_unset") to perform diffs between trees who are similar to `index` in a typical source code repository.

Currently, this allocates about 10MB for every 10k files in `index`, and a minimum of 4KB.

[Source](https://docs.rs/gix/latest/src/gix/repository/cache.rs.html#44-52)[§](#impl-Repository-2)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Handling of InMemory object writing

[Source](https://docs.rs/gix/latest/src/gix/repository/cache.rs.html#48-51)

#### pub fn [with\_object\_memory](#method.with_object_memory)(self) -> Self

When writing objects, keep them in memory instead of writing them to disk. This makes any change to the object database non-persisting, while keeping the view to the object database consistent for this instance.

[Source](https://docs.rs/gix/latest/src/gix/repository/checkout.rs.html#3-14)[§](#impl-Repository-3)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/checkout.rs.html#8-13)

#### pub fn [checkout\_options](#method.checkout_options)( &self, attributes\_source: [Source](https://docs.rs/gix/latest/gix/worktree/stack/state/attributes/enum.Source.html "enum gix::worktree::stack::state::attributes::Source"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Options](https://docs.rs/gix-worktree-state/0.19.0/x86_64-unknown-linux-gnu/gix_worktree_state/checkout/struct.Options.html "struct gix_worktree_state::checkout::Options"), [Error](https://docs.rs/gix/latest/gix/config/checkout_options/enum.Error.html "enum gix::config::checkout_options::Error")\>

Available on **crate feature `worktree-mutation`** only.

Return options that can be used to drive a low-level checkout operation. Use `attributes_source` to determine where `.gitattributes` files should be read from, which depends on the presence of a worktree to begin with. Here, typically this value would be [`gix_worktree::stack::state::attributes::Source::IdMapping`](https://docs.rs/gix/latest/gix/worktree/stack/state/attributes/enum.Source.html#variant.IdMapping "variant gix::worktree::stack::state::attributes::Source::IdMapping")

[Source](https://docs.rs/gix/latest/src/gix/repository/config/branch.rs.html#18-256)[§](#impl-Repository-4)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Query configuration related to branches.

[Source](https://docs.rs/gix/latest/src/gix/repository/config/branch.rs.html#25-27)

#### pub fn [branch\_names](#method.branch_names)(&self) -> [BTreeSet](https://doc.rust-lang.org/nightly/alloc/collections/btree/set/struct.BTreeSet.html "struct alloc::collections::btree::set::BTreeSet")<&[str](https://doc.rust-lang.org/nightly/std/primitive.str.html)\>

Return a set of unique short branch names for which custom configuration exists in the configuration, if we deem them [trustworthy](https://docs.rs/gix/latest/gix/open/struct.Options.html#method.filter_config_section "method gix::open::Options::filter_config_section").

###### [§](#note)Note

Branch names that have illformed UTF-8 will silently be skipped.

[Source](https://docs.rs/gix/latest/src/gix/repository/config/branch.rs.html#44-92)

#### pub fn [branch\_remote\_ref\_name](#method.branch_remote_ref_name)( &self, name: &[FullNameRef](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.FullNameRef.html "struct gix_ref::FullNameRef"), direction: [Direction](https://docs.rs/gix/latest/gix/remote/enum.Direction.html "enum gix::remote::Direction"), ) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Cow](https://doc.rust-lang.org/nightly/alloc/borrow/enum.Cow.html "enum alloc::borrow::Cow")<'\_, [FullNameRef](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.FullNameRef.html "struct gix_ref::FullNameRef")\>, [Error](https://docs.rs/gix/latest/gix/repository/branch_remote_ref_name/enum.Error.html "enum gix::repository::branch_remote_ref_name::Error")\>>

Returns the validated reference name of the upstream branch on the remote associated with the given `name`, which will be used when _merging_. The returned value corresponds to the `branch.<short_branch_name>.merge` configuration key for [`remote::Direction::Fetch`](https://docs.rs/gix/latest/gix/remote/enum.Direction.html#variant.Fetch "variant gix::remote::Direction::Fetch"). For the [push direction](https://docs.rs/gix/latest/gix/remote/enum.Direction.html#variant.Push "variant gix::remote::Direction::Push") the Git configuration is used for a variety of different outcomes, similar to what would happen when running `git push <name>`.

Returns `None` if there is nothing configured, or if no remote or remote ref is configured.

###### [§](#note-1)Note

The returned name refers to what Git calls upstream branch (as opposed to upstream _tracking_ branch). The value is also fast to retrieve compared to its tracking branch.

See also [`Reference::remote_ref_name()`](https://docs.rs/gix/latest/gix/struct.Reference.html#method.remote_ref_name "method gix::Reference::remote_ref_name").

[Source](https://docs.rs/gix/latest/src/gix/repository/config/branch.rs.html#112-131)

#### pub fn [branch\_remote\_tracking\_ref\_name](#method.branch_remote_tracking_ref_name)( &self, name: &[FullNameRef](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.FullNameRef.html "struct gix_ref::FullNameRef"), direction: [Direction](https://docs.rs/gix/latest/gix/remote/enum.Direction.html "enum gix::remote::Direction"), ) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Cow](https://doc.rust-lang.org/nightly/alloc/borrow/enum.Cow.html "enum alloc::borrow::Cow")<'\_, [FullNameRef](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.FullNameRef.html "struct gix_ref::FullNameRef")\>, [Error](https://docs.rs/gix/latest/gix/repository/branch_remote_tracking_ref_name/enum.Error.html "enum gix::repository::branch_remote_tracking_ref_name::Error")\>>

Return the validated name of the reference that tracks the corresponding reference of `name` on the remote for `direction`. Note that a branch with that name might not actually exist.

*   with `remote` being [remote::Direction::Fetch](https://docs.rs/gix/latest/gix/remote/enum.Direction.html#variant.Fetch "variant gix::remote::Direction::Fetch"), we return the tracking branch that is on the destination side of a `src:dest` refspec. For instance, with `name` being `main` and the default refspec `refs/heads/*:refs/remotes/origin/*`, `refs/heads/main` would match and produce `refs/remotes/origin/main`.
*   with `remote` being [remote::Direction::Push](https://docs.rs/gix/latest/gix/remote/enum.Direction.html#variant.Push "variant gix::remote::Direction::Push"), we return the tracking branch that corresponds to the remote branch that we would push to. For instance, with `name` being `main` and no setup at all, we would push to `refs/heads/main` on the remote. And that one would be fetched matching the `refs/heads/*:refs/remotes/origin/*` fetch refspec, hence `refs/remotes/origin/main` is returned. Note that `push` refspecs can be used to map `main` to `other` (using a push refspec `refs/heads/main:refs/heads/other`), which would then lead to `refs/remotes/origin/other` to be returned instead.

Note that if there is an ambiguity, that is if `name` maps to multiple tracking branches, the first matching mapping is returned, according to the order in which the fetch or push refspecs occur in the configuration file.

See also [`Reference::remote_tracking_ref_name()`](https://docs.rs/gix/latest/gix/struct.Reference.html#method.remote_tracking_ref_name "method gix::Reference::remote_tracking_ref_name").

[Source](https://docs.rs/gix/latest/src/gix/repository/config/branch.rs.html#142-198)

#### pub fn [upstream\_branch\_and\_remote\_for\_tracking\_branch](#method.upstream_branch_and_remote_for_tracking_branch)( &self, tracking\_branch: &[FullNameRef](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.FullNameRef.html "struct gix_ref::FullNameRef"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<([FullName](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.FullName.html "struct gix_ref::FullName"), [Remote](https://docs.rs/gix/latest/gix/struct.Remote.html "struct gix::Remote")<'\_>)>, [Error](https://docs.rs/gix/latest/gix/repository/upstream_branch_and_remote_name_for_tracking_branch/enum.Error.html "enum gix::repository::upstream_branch_and_remote_name_for_tracking_branch::Error")\>

Given a local `tracking_branch` name, find the remote that maps to it along with the name of the branch on the side of the remote, also called upstream branch.

Return `Ok(None)` if there is no remote with fetch-refspecs that would match `tracking_branch` on the right-hand side, or `Err` if the matches were ambiguous.

###### [§](#limitations)Limitations

A single valid mapping is required as fine-grained matching isn’t implemented yet. This means that

[Source](https://docs.rs/gix/latest/src/gix/repository/config/branch.rs.html#214-230)

#### pub fn [branch\_remote\_name](#method.branch_remote_name)<'a>( &self, short\_branch\_name: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<&'a [BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>, direction: [Direction](https://docs.rs/gix/latest/gix/remote/enum.Direction.html "enum gix::remote::Direction"), ) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Name](https://docs.rs/gix/latest/gix/remote/enum.Name.html "enum gix::remote::Name")<'\_>>

Returns the unvalidated name of the remote associated with the given `short_branch_name`, typically `main` instead of `refs/heads/main`. In some cases, the returned name will be an URL. Returns `None` if the remote was not found or if the name contained illformed UTF-8.

*   if `direction` is [remote::Direction::Fetch](https://docs.rs/gix/latest/gix/remote/enum.Direction.html#variant.Fetch "variant gix::remote::Direction::Fetch"), we will query the `branch.<short_name>.remote` configuration.
*   if `direction` is [remote::Direction::Push](https://docs.rs/gix/latest/gix/remote/enum.Direction.html#variant.Push "variant gix::remote::Direction::Push"), the push remote will be queried by means of `branch.<short_name>.pushRemote` or `remote.pushDefault` as fallback.

See also [`Reference::remote_name()`](https://docs.rs/gix/latest/gix/struct.Reference.html#method.remote_name "method gix::Reference::remote_name") for a more typesafe version to be used when a `Reference` is available.

`short_branch_name` can typically be obtained by [shortening a full branch name](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.FullNameRef.html#method.shorten "method gix_ref::FullNameRef::shorten").

[Source](https://docs.rs/gix/latest/src/gix/repository/config/branch.rs.html#237-255)

#### pub fn [branch\_remote](#method.branch_remote)<'a>( &self, short\_branch\_name: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<&'a [BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>, direction: [Direction](https://docs.rs/gix/latest/gix/remote/enum.Direction.html "enum gix::remote::Direction"), ) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Remote](https://docs.rs/gix/latest/gix/struct.Remote.html "struct gix::Remote")<'\_>, [Error](https://docs.rs/gix/latest/gix/remote/find/existing/enum.Error.html "enum gix::remote::find::existing::Error")\>>

Like [`branch_remote_name(…)`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.branch_remote_name "method gix::Repository::branch_remote_name"), but returns a [Remote](https://docs.rs/gix/latest/gix/struct.Remote.html "struct gix::Remote"). `short_branch_name` is the name to use for looking up `branch.<short_branch_name>.*` values in the configuration.

See also [`Reference::remote()`](https://docs.rs/gix/latest/gix/struct.Reference.html#method.remote "method gix::Reference::remote").

[Source](https://docs.rs/gix/latest/src/gix/repository/config/remote.rs.html#10-54)[§](#impl-Repository-5)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Query configuration related to remotes.

[Source](https://docs.rs/gix/latest/src/gix/repository/config/remote.rs.html#13-24)

#### pub fn [remote\_names](#method.remote_names)(&self) -> [Names](https://docs.rs/gix/latest/gix/remote/type.Names.html "type gix::remote::Names")<'\_>

Returns a sorted list unique of symbolic names of remotes that we deem [trustworthy](https://docs.rs/gix/latest/gix/open/struct.Options.html#method.filter_config_section "method gix::open::Options::filter_config_section").

[Source](https://docs.rs/gix/latest/src/gix/repository/config/remote.rs.html#34-53)

#### pub fn [remote\_default\_name](#method.remote_default_name)(&self, direction: [Direction](https://docs.rs/gix/latest/gix/remote/enum.Direction.html "enum gix::remote::Direction")) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Cow](https://doc.rust-lang.org/nightly/alloc/borrow/enum.Cow.html "enum alloc::borrow::Cow")<'\_, [BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>>

Obtain the branch-independent name for a remote for use in the given `direction`, or `None` if it could not be determined.

For _fetching_, use the only configured remote, or default to `origin` if it exists. For _pushing_, use the `remote.pushDefault` trusted configuration key, or fall back to the rules for _fetching_.

##### [§](#notes)Notes

It’s up to the caller to determine what to do if the current `head` is unborn or detached.

[Source](https://docs.rs/gix/latest/src/gix/repository/config/transport.rs.html#6-444)[§](#impl-Repository-6)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/config/transport.rs.html#24-443)

#### pub fn [transport\_options](#method.transport_options)<'a>( &self, url: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<&'a [BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>, remote\_name: [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<&[BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Box](https://doc.rust-lang.org/nightly/alloc/boxed/struct.Box.html "struct alloc::boxed::Box")<dyn [Any](https://doc.rust-lang.org/nightly/core/any/trait.Any.html "trait core::any::Any")\>>, [Error](https://docs.rs/gix/latest/gix/config/transport/enum.Error.html "enum gix::config::transport::Error")\>

Available on **crate features `blocking-network-client` or `async-network-client`** only.

Produce configuration suitable for `url`, as differentiated by its protocol/scheme, to be passed to a transport instance via [configure()](https://docs.rs/gix-transport/0.47.0/x86_64-unknown-linux-gnu/gix_transport/client/traits/trait.TransportWithoutIO.html#tymethod.configure "method gix_transport::client::traits::TransportWithoutIO::configure") (via `&**config` to pass the contained `Any` and not the `Box`). `None` is returned if there is no known configuration. If `remote_name` is not `None`, the remote’s name may contribute to configuration overrides, typically for the HTTP transport.

Note that the caller may cast the instance themselves to modify it before passing it on.

For transports that support proxy authentication, the [default authentication method](https://docs.rs/gix/latest/gix/config/struct.Snapshot.html#method.credential_helpers "method gix::config::Snapshot::credential_helpers") will be used with the url of the proxy if it contains a user name.

[Source](https://docs.rs/gix/latest/src/gix/repository/config/mod.rs.html#6-137)[§](#impl-Repository-7)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

General Configuration

[Source](https://docs.rs/gix/latest/src/gix/repository/config/mod.rs.html#8-10)

#### pub fn [config\_snapshot](#method.config_snapshot)(&self) -> [Snapshot](https://docs.rs/gix/latest/gix/config/struct.Snapshot.html "struct gix::config::Snapshot")<'\_>

Return a snapshot of the configuration as seen upon opening the repository.

[Source](https://docs.rs/gix/latest/src/gix/repository/config/mod.rs.html#17-23)

#### pub fn [config\_snapshot\_mut](#method.config_snapshot_mut)(&mut self) -> [SnapshotMut](https://docs.rs/gix/latest/gix/config/struct.SnapshotMut.html "struct gix::config::SnapshotMut")<'\_>

Return a mutable snapshot of the configuration as seen upon opening the repository, starting a transaction. When the returned instance is dropped, it is applied in full, even if the reason for the drop is an error.

Note that changes to the configuration are in-memory only and are observed only this instance of the [`Repository`](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository").

[Source](https://docs.rs/gix/latest/src/gix/repository/config/mod.rs.html#28-30)

#### pub fn [filesystem\_options](#method.filesystem_options)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Capabilities](https://docs.rs/gix-fs/0.15.0/x86_64-unknown-linux-gnu/gix_fs/struct.Capabilities.html "struct gix_fs::Capabilities"), [Error](https://docs.rs/gix/latest/gix/config/boolean/type.Error.html "type gix::config::boolean::Error")\>

Return filesystem options as retrieved from the repository configuration.

Note that these values have not been [probed](https://docs.rs/gix-fs/0.15.0/x86_64-unknown-linux-gnu/gix_fs/struct.Capabilities.html#method.probe "associated function gix_fs::Capabilities::probe").

[Source](https://docs.rs/gix/latest/src/gix/repository/config/mod.rs.html#36-38)

#### pub fn [stat\_options](#method.stat_options)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Options](https://docs.rs/gix/latest/gix/index/entry/stat/struct.Options.html "struct gix::index::entry::stat::Options"), [Error](https://docs.rs/gix/latest/gix/config/stat_options/enum.Error.html "enum gix::config::stat_options::Error")\>

Available on **crate feature `index`** only.

Return filesystem options on how to perform stat-checks, typically in relation to the index.

Note that these values have not been [probed](https://docs.rs/gix-fs/0.15.0/x86_64-unknown-linux-gnu/gix_fs/struct.Capabilities.html#method.probe "associated function gix_fs::Capabilities::probe").

[Source](https://docs.rs/gix/latest/src/gix/repository/config/mod.rs.html#41-43)

#### pub fn [open\_options](#method.open_options)(&self) -> &[Options](https://docs.rs/gix/latest/gix/open/struct.Options.html "struct gix::open::Options")

The options used to open the repository.

[Source](https://docs.rs/gix/latest/src/gix/repository/config/mod.rs.html#47-49)

#### pub fn [big\_file\_threshold](#method.big_file_threshold)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[u64](https://doc.rust-lang.org/nightly/std/primitive.u64.html), [Error](https://docs.rs/gix/latest/gix/config/unsigned_integer/type.Error.html "type gix::config::unsigned_integer::Error")\>

Return the big-file threshold above which Git will not perform a diff anymore or try to delta-diff packs, as configured by `core.bigFileThreshold`, or the default value.

[Source](https://docs.rs/gix/latest/src/gix/repository/config/mod.rs.html#53-81)

#### pub fn [ssh\_connect\_options](#method.ssh_connect_options)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Options](https://docs.rs/gix-transport/0.47.0/x86_64-unknown-linux-gnu/gix_transport/client/blocking_io/ssh/connect/struct.Options.html "struct gix_transport::client::blocking_io::ssh::connect::Options"), [Error](https://docs.rs/gix/latest/gix/config/ssh_connect_options/struct.Error.html "struct gix::config::ssh_connect_options::Error")\>

Available on **crate feature `blocking-network-client`** only.

Obtain options for use when connecting via `ssh`.

[Source](https://docs.rs/gix/latest/src/gix/repository/config/mod.rs.html#86-123)

#### pub fn [command\_context](#method.command_context)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Context](https://docs.rs/gix/latest/gix/diff/command/struct.Context.html "struct gix::diff::command::Context"), [Error](https://docs.rs/gix/latest/gix/config/command_context/enum.Error.html "enum gix::config::command_context::Error")\>

Available on **crate feature `attributes`** only.

Return the context to be passed to any spawned program that is supposed to interact with the repository, like hooks or filters.

[Source](https://docs.rs/gix/latest/src/gix/repository/config/mod.rs.html#126-128)

#### pub fn [object\_hash](#method.object_hash)(&self) -> [Kind](https://docs.rs/gix/latest/gix/index/hash/enum.Kind.html "enum gix::index::hash::Kind")

The kind of object hash the repository is configured to use.

[Source](https://docs.rs/gix/latest/src/gix/repository/config/mod.rs.html#134-136)

#### pub fn [diff\_algorithm](#method.diff_algorithm)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Algorithm](https://docs.rs/gix/latest/gix/diff/blob/enum.Algorithm.html "enum gix::diff::blob::Algorithm"), [Error](https://docs.rs/gix/latest/gix/config/diff/algorithm/enum.Error.html "enum gix::config::diff::algorithm::Error")\>

Available on **crate feature `blob-diff`** only.

Return the algorithm to perform diffs or merges with.

In case of merges, a diff is performed under the hood in order to learn which hunks need merging.

[Source](https://docs.rs/gix/latest/src/gix/repository/diff.rs.html#9-90)[§](#impl-Repository-8)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Diff-utilities

[Source](https://docs.rs/gix/latest/src/gix/repository/diff.rs.html#20-40)

#### pub fn [diff\_resource\_cache](#method.diff_resource_cache)( &self, mode: [Mode](https://docs.rs/gix/latest/gix/diff/blob/pipeline/enum.Mode.html "enum gix::diff::blob::pipeline::Mode"), worktree\_roots: [WorktreeRoots](https://docs.rs/gix/latest/gix/diff/blob/pipeline/struct.WorktreeRoots.html "struct gix::diff::blob::pipeline::WorktreeRoots"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Platform](https://docs.rs/gix/latest/gix/diff/blob/struct.Platform.html "struct gix::diff::blob::Platform"), [Error](https://docs.rs/gix/latest/gix/repository/diff_resource_cache/enum.Error.html "enum gix::repository::diff_resource_cache::Error")\>

Available on **crate feature `blob-diff`** only.

Create a resource cache for diffable objects, and configured with everything it needs to know to perform diffs faithfully just like `git` would. `mode` controls what version of a resource should be diffed. `worktree_roots` determine if files can be read from the worktree, where each side of the diff operation can be represented by its own worktree root. `.gitattributes` are automatically read from the worktree if at least one worktree is present.

Note that attributes will always be obtained from the current `HEAD` index even if the resources being diffed might live in another tree. Further, if one of the `worktree_roots` are set, attributes will also be read from the worktree. Otherwise, it will be skipped and attributes are read from the index tree instead.

[Source](https://docs.rs/gix/latest/src/gix/repository/diff.rs.html#50-79)

#### pub fn [diff\_tree\_to\_tree](#method.diff_tree_to_tree)<'a, 'old\_repo: 'a, 'new\_repo: 'a>( &self, old\_tree: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<&'a [Tree](https://docs.rs/gix/latest/gix/struct.Tree.html "struct gix::Tree")<'old\_repo>>>, new\_tree: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<&'a [Tree](https://docs.rs/gix/latest/gix/struct.Tree.html "struct gix::Tree")<'new\_repo>>>, options: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Options](https://docs.rs/gix/latest/gix/diff/struct.Options.html "struct gix::diff::Options")\>>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[ChangeDetached](https://docs.rs/gix/latest/gix/diff/tree_with_rewrites/enum.Change.html "enum gix::diff::tree_with_rewrites::Change")\>, [Error](https://docs.rs/gix/latest/gix/repository/diff_tree_to_tree/enum.Error.html "enum gix::repository::diff_tree_to_tree::Error")\>

Available on **crate feature `blob-diff`** only.

Produce the changes that would need to be applied to `old_tree` to create `new_tree`. If `options` are unset, they will be filled in according to the git configuration of this repository, and with [full paths being tracked](https://docs.rs/gix/latest/gix/diff/struct.Options.html#method.track_path "method gix::diff::Options::track_path") as well, which typically means that rewrite tracking might be disabled if done so explicitly by the user. If `options` are set, the user can take full control over the settings.

Note that this method exists to evoke similarity to `git2`, and makes it easier to fully control diff settings. A more fluent version [may be used as well](https://docs.rs/gix/latest/gix/struct.Tree.html#method.changes "method gix::Tree::changes").

[Source](https://docs.rs/gix/latest/src/gix/repository/diff.rs.html#84-89)

#### pub fn [diff\_resource\_cache\_for\_tree\_diff](#method.diff_resource_cache_for_tree_diff)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Platform](https://docs.rs/gix/latest/gix/diff/blob/struct.Platform.html "struct gix::diff::blob::Platform"), [Error](https://docs.rs/gix/latest/gix/repository/diff_resource_cache/enum.Error.html "enum gix::repository::diff_resource_cache::Error")\>

Available on **crate feature `blob-diff`** only.

Return a resource cache suitable for diffing blobs from trees directly, where no worktree checkout exists.

For more control, see [`diff_resource_cache()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.diff_resource_cache "method gix::Repository::diff_resource_cache").

[Source](https://docs.rs/gix/latest/src/gix/repository/dirwalk.rs.html#11-137)[§](#impl-Repository-9)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/dirwalk.rs.html#15-17)

#### pub fn [dirwalk\_options](#method.dirwalk_options)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Options](https://docs.rs/gix/latest/gix/dirwalk/struct.Options.html "struct gix::dirwalk::Options"), [Error](https://docs.rs/gix/latest/gix/config/boolean/type.Error.html "type gix::config::boolean::Error")\>

Available on **crate feature `dirwalk`** only.

Return default options suitable for performing a directory walk on this repository.

Used in conjunction with [`dirwalk()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.dirwalk "method gix::Repository::dirwalk")

[Source](https://docs.rs/gix/latest/src/gix/repository/dirwalk.rs.html#34-115)

#### pub fn [dirwalk](#method.dirwalk)( &self, index: &[State](https://docs.rs/gix/latest/gix/index/struct.State.html "struct gix::index::State"), patterns: impl [IntoIterator](https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")<Item = impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>>, should\_interrupt: &[AtomicBool](https://doc.rust-lang.org/nightly/core/sync/atomic/struct.AtomicBool.html "struct core::sync::atomic::AtomicBool"), options: [Options](https://docs.rs/gix/latest/gix/dirwalk/struct.Options.html "struct gix::dirwalk::Options"), delegate: &mut dyn [Delegate](https://docs.rs/gix-dir/0.14.1/x86_64-unknown-linux-gnu/gix_dir/walk/trait.Delegate.html "trait gix_dir::walk::Delegate"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Outcome](https://docs.rs/gix/latest/gix/dirwalk/struct.Outcome.html "struct gix::dirwalk::Outcome")<'\_>, [Error](https://docs.rs/gix/latest/gix/dirwalk/enum.Error.html "enum gix::dirwalk::Error")\>

Available on **crate feature `dirwalk`** only.

Perform a directory walk configured with `options` under control of the `delegate`. Use `patterns` to further filter entries. `should_interrupt` is polled to see if an interrupt is requested, causing an error to be returned instead.

The `index` is used to determine if entries are tracked, and for excludes and attributes lookup. Note that items will only count as tracked if they have the [`gix_index::entry::Flags::UPTODATE`](https://docs.rs/gix/latest/gix/index/entry/struct.Flags.html#associatedconstant.UPTODATE "associated constant gix::index::entry::Flags::UPTODATE") flag set.

Note that dirwalks for the purpose of deletion will be initialized with the worktrees of this repository if they fall into the working directory of this repository as well to mark them as `tracked`. That way it’s hard to accidentally flag them for deletion. This is intentionally not the case when deletion is not intended so they look like untracked repositories instead.

See [`gix_dir::walk::delegate::Collect`](https://docs.rs/gix-dir/0.14.1/x86_64-unknown-linux-gnu/gix_dir/walk/delegate/struct.Collect.html "struct gix_dir::walk::delegate::Collect") for a delegate that collects all seen entries.

[Source](https://docs.rs/gix/latest/src/gix/repository/dirwalk.rs.html#122-136)

#### pub fn [dirwalk\_iter](#method.dirwalk_iter)( &self, index: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[IndexPersistedOrInMemory](https://docs.rs/gix/latest/gix/worktree/enum.IndexPersistedOrInMemory.html "enum gix::worktree::IndexPersistedOrInMemory")\>, patterns: impl [IntoIterator](https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")<Item = impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[BString](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BString.html "struct gix::diff::object::bstr::BString")\>>, should\_interrupt: OwnedOrStaticAtomicBool, options: [Options](https://docs.rs/gix/latest/gix/dirwalk/struct.Options.html "struct gix::dirwalk::Options"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Iter](https://docs.rs/gix/latest/gix/dirwalk/struct.Iter.html "struct gix::dirwalk::Iter"), [Error](https://docs.rs/gix/latest/gix/dirwalk/iter/enum.Error.html "enum gix::dirwalk::iter::Error")\>

Available on **crate feature `dirwalk`** only.

Create an iterator over a running traversal, which stops if the iterator is dropped. All arguments are the same as in [`dirwalk()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.dirwalk "method gix::Repository::dirwalk").

`should_interrupt` should be set to `Default::default()` if it is supposed to be unused. Otherwise, it can be created by passing a `&'static AtomicBool`, `&Arc<AtomicBool>` or `Arc<AtomicBool>`.

[Source](https://docs.rs/gix/latest/src/gix/repository/filter.rs.html#24-64)[§](#impl-Repository-10)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/filter.rs.html#39-63)

#### pub fn [filter\_pipeline](#method.filter_pipeline)( &self, tree\_if\_bare: [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<([Pipeline](https://docs.rs/gix/latest/gix/filter/struct.Pipeline.html "struct gix::filter::Pipeline")<'\_>, [IndexPersistedOrInMemory](https://docs.rs/gix/latest/gix/worktree/enum.IndexPersistedOrInMemory.html "enum gix::worktree::IndexPersistedOrInMemory")), [Error](https://docs.rs/gix/latest/gix/repository/filter/pipeline/enum.Error.html "enum gix::repository::filter::pipeline::Error")\>

Available on **crate feature `attributes`** only.

Configure a pipeline for converting byte buffers to the worktree representation, and byte streams to the git-internal representation. Also return the index that was used when initializing the pipeline as it may be useful when calling [convert\_to\_git()](https://docs.rs/gix/latest/gix/filter/struct.Pipeline.html#method.convert_to_git "method gix::filter::Pipeline::convert_to_git"). Bare repositories will either use `HEAD^{tree}` for accessing all relevant worktree files or the given `tree_if_bare`.

Note that this is considered a primitive as it operates on data directly and will not have permanent effects. We also return the index that was used to configure the attributes cache (for accessing `.gitattributes`), which can be reused after it was possibly created from a tree, an expensive operation.

###### [§](#performance)Performance

Note that when in a repository with worktree, files in the worktree will be read with priority, which causes at least a stat each time the directory is changed. This can be expensive if access isn’t in sorted order, which would cause more then necessary stats: one per directory.

[Source](https://docs.rs/gix/latest/src/gix/repository/freelist.rs.html#75-99)[§](#impl-Repository-11)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Freelist configuration

The free-list is an internal and ‘transparent’ mechanism for obtaining and re-using memory buffers when reading objects. That way, trashing is avoided as buffers are re-used and re-written.

However, there are circumstances when releasing memory early is preferred, for instance on the server side.

Also note that the free-list isn’t cloned, so each clone of this instance starts with an empty one.

[Source](https://docs.rs/gix/latest/src/gix/repository/freelist.rs.html#78-82)

#### pub fn [empty\_reusable\_buffer](#method.empty_reusable_buffer)(&self) -> [Buffer](https://docs.rs/gix/latest/gix/repository/freelist/struct.Buffer.html "struct gix::repository::freelist::Buffer")<'\_>

Return an empty buffer which is tied to this repository instance, and reuse its memory allocation by keeping it around even after it drops.

[Source](https://docs.rs/gix/latest/src/gix/repository/freelist.rs.html#88-92)

#### pub fn [set\_freelist](#method.set_freelist)( &mut self, list: [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html)\>>>, ) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html)\>>>

Set the currently used freelist to `list`. If `None`, it will be disabled entirely.

Return the currently previously allocated free-list, a list of reusable buffers typically used when reading objects. May be `None` if there was no free-list.

[Source](https://docs.rs/gix/latest/src/gix/repository/freelist.rs.html#95-98)

#### pub fn [without\_freelist](#method.without_freelist)(self) -> Self

A builder method to disable the free-list on a newly created instance.

[Source](https://docs.rs/gix/latest/src/gix/repository/graph.rs.html#1-45)[§](#impl-Repository-12)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/graph.rs.html#15-20)

#### pub fn [revision\_graph](#method.revision_graph)<'cache, T>( &self, cache: [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<&'cache [Graph](https://docs.rs/gix-commitgraph/0.28.0/x86_64-unknown-linux-gnu/gix_commitgraph/struct.Graph.html "struct gix_commitgraph::Graph")\>, ) -> [Graph](https://docs.rs/gix-revwalk/0.20.1/x86_64-unknown-linux-gnu/gix_revwalk/struct.Graph.html "struct gix_revwalk::Graph")<'\_, 'cache, T>

Create a graph data-structure capable of accelerating graph traversals and storing state of type `T` with each commit it encountered.

Note that the `cache` will be used if present, and it’s best obtained with [`commit_graph_if_enabled()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.commit_graph_if_enabled "method gix::Repository::commit_graph_if_enabled").

Note that a commitgraph is only allowed to be used if `core.commitGraph` is true (the default), and that configuration errors are ignored as well.

###### [§](#performance-1)Performance

Note that the [Graph](https://docs.rs/gix-revwalk/0.20.1/x86_64-unknown-linux-gnu/gix_revwalk/struct.Graph.html "struct gix_revwalk::Graph") can be sensitive to various object database settings that may affect the performance of the commit walk.

[Source](https://docs.rs/gix/latest/src/gix/repository/graph.rs.html#27-29)

#### pub fn [commit\_graph](#method.commit_graph)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Graph](https://docs.rs/gix-commitgraph/0.28.0/x86_64-unknown-linux-gnu/gix_commitgraph/struct.Graph.html "struct gix_commitgraph::Graph"), [Error](https://docs.rs/gix-commitgraph/0.28.0/x86_64-unknown-linux-gnu/gix_commitgraph/init/enum.Error.html "enum gix_commitgraph::init::Error")\>

Return a cache for commits and their graph structure, as managed by `git commit-graph`, for accelerating commit walks on a low level.

Note that [`revision_graph()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.revision_graph "method gix::Repository::revision_graph") should be preferred for general purpose walks that don’t rely on the actual commit cache to be present, while leveraging the commit-graph if possible.

[Source](https://docs.rs/gix/latest/src/gix/repository/graph.rs.html#32-44)

#### pub fn [commit\_graph\_if\_enabled](#method.commit_graph_if_enabled)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Graph](https://docs.rs/gix-commitgraph/0.28.0/x86_64-unknown-linux-gnu/gix_commitgraph/struct.Graph.html "struct gix_commitgraph::Graph")\>, [Error](https://docs.rs/gix/latest/gix/repository/commit_graph_if_enabled/enum.Error.html "enum gix::repository::commit_graph_if_enabled::Error")\>

Return a newly opened commit-graph if it is available _and_ enabled in the Git configuration.

[Source](https://docs.rs/gix/latest/src/gix/repository/identity.rs.html#17-67)[§](#impl-Repository-13)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Identity handling.

#### [§](#deviation)Deviation

There is no notion of a default user like in git, and instead failing to provide a user is fatal. That way, we enforce correctness and force application developers to take care of this issue which can be done in various ways, for instance by setting `gitoxide.committer.nameFallback` and similar.

[Source](https://docs.rs/gix/latest/src/gix/repository/identity.rs.html#30-44)

#### pub fn [committer](#method.committer)(&self) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[SignatureRef](https://docs.rs/gix-actor/0.35.1/x86_64-unknown-linux-gnu/gix_actor/struct.SignatureRef.html "struct gix_actor::SignatureRef")<'\_>, [Error](https://docs.rs/gix/latest/gix/config/time/type.Error.html "type gix::config::time::Error")\>>

Return the committer as configured by this repository, which is determined by…

*   …the git configuration `committer.name|email`…
*   …the `GIT_COMMITTER_(NAME|EMAIL|DATE)` environment variables…
*   …the configuration for `user.name|email` as fallback…

…and in that order, or `None` if no committer name or email was configured, or `Some(Err(…))` if the committer date could not be parsed.

##### [§](#note-2)Note

The values are cached when the repository is instantiated.

[Source](https://docs.rs/gix/latest/src/gix/repository/identity.rs.html#57-66)

#### pub fn [author](#method.author)(&self) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[SignatureRef](https://docs.rs/gix-actor/0.35.1/x86_64-unknown-linux-gnu/gix_actor/struct.SignatureRef.html "struct gix_actor::SignatureRef")<'\_>, [Error](https://docs.rs/gix/latest/gix/config/time/type.Error.html "type gix::config::time::Error")\>>

Return the author as configured by this repository, which is determined by…

*   …the git configuration `author.name|email`…
*   …the `GIT_AUTHOR_(NAME|EMAIL|DATE)` environment variables…
*   …the configuration for `user.name|email` as fallback…

…and in that order, or `None` if there was nothing configured.

##### [§](#note-3)Note

The values are cached when the repository is instantiated.

[Source](https://docs.rs/gix/latest/src/gix/repository/index.rs.html#8-157)[§](#impl-Repository-14)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Index access

[Source](https://docs.rs/gix/latest/src/gix/repository/index.rs.html#13-42)

#### pub fn [open\_index](#method.open_index)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[File](https://docs.rs/gix/latest/gix/index/struct.File.html "struct gix::index::File"), [Error](https://docs.rs/gix/latest/gix/worktree/open_index/enum.Error.html "enum gix::worktree::open_index::Error")\>

Available on **crate feature `index`** only.

Open a new copy of the index file and decode it entirely.

It will use the `index.threads` configuration key to learn how many threads to use. Note that it may fail if there is no index.

[Source](https://docs.rs/gix/latest/src/gix/repository/index.rs.html#53-66)

#### pub fn [index](#method.index)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Index](https://docs.rs/gix/latest/gix/worktree/type.Index.html "type gix::worktree::Index"), [Error](https://docs.rs/gix/latest/gix/worktree/open_index/enum.Error.html "enum gix::worktree::open_index::Error")\>

Available on **crate feature `index`** only.

Return a shared worktree index which is updated automatically if the in-memory snapshot has become stale as the underlying file on disk has changed.

###### [§](#notes-1)Notes

*   This will fail if the file doesn’t exist, like in a newly initialized repository. If that is the case, use [index\_or\_empty()](https://docs.rs/gix/latest/gix/struct.Repository.html#method.index_or_empty "method gix::Repository::index_or_empty") or [try\_index()](https://docs.rs/gix/latest/gix/struct.Repository.html#method.try_index "method gix::Repository::try_index") instead.

The index file is shared across all clones of this repository.

[Source](https://docs.rs/gix/latest/src/gix/repository/index.rs.html#69-76)

#### pub fn [index\_or\_empty](#method.index_or_empty)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Index](https://docs.rs/gix/latest/gix/worktree/type.Index.html "type gix::worktree::Index"), [Error](https://docs.rs/gix/latest/gix/worktree/open_index/enum.Error.html "enum gix::worktree::open_index::Error")\>

Available on **crate feature `index`** only.

Return the shared worktree index if present, or return a new empty one which has an association to the place where the index would be.

[Source](https://docs.rs/gix/latest/src/gix/repository/index.rs.html#82-96)

#### pub fn [try\_index](#method.try_index)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Index](https://docs.rs/gix/latest/gix/worktree/type.Index.html "type gix::worktree::Index")\>, [Error](https://docs.rs/gix/latest/gix/worktree/open_index/enum.Error.html "enum gix::worktree::open_index::Error")\>

Available on **crate feature `index`** only.

Return a shared worktree index which is updated automatically if the in-memory snapshot has become stale as the underlying file on disk has changed, or `None` if no such file exists.

The index file is shared across all clones of this repository.

[Source](https://docs.rs/gix/latest/src/gix/repository/index.rs.html#108-118)

#### pub fn [index\_or\_load\_from\_head](#method.index_or_load_from_head)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[IndexPersistedOrInMemory](https://docs.rs/gix/latest/gix/worktree/enum.IndexPersistedOrInMemory.html "enum gix::worktree::IndexPersistedOrInMemory"), [Error](https://docs.rs/gix/latest/gix/repository/index_or_load_from_head/enum.Error.html "enum gix::repository::index_or_load_from_head::Error")\>

Available on **crate feature `index`** only.

Open the persisted worktree index or generate it from the current `HEAD^{tree}` to live in-memory only.

Use this method to get an index in any repository, even bare ones that don’t have one naturally.

###### [§](#note-4)Note

*   The locally stored index is not guaranteed to represent `HEAD^{tree}` if this repository is bare - bare repos don’t naturally have an index and if an index is present it must have been generated by hand.
*   This method will fail on unborn repositories as `HEAD` doesn’t point to a reference yet, which is needed to resolve the revspec. If that is a concern, use [`Self::index_or_load_from_head_or_empty()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.index_or_load_from_head_or_empty "method gix::Repository::index_or_load_from_head_or_empty") instead.

[Source](https://docs.rs/gix/latest/src/gix/repository/index.rs.html#125-141)

#### pub fn [index\_or\_load\_from\_head\_or\_empty](#method.index_or_load_from_head_or_empty)( &self, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[IndexPersistedOrInMemory](https://docs.rs/gix/latest/gix/worktree/enum.IndexPersistedOrInMemory.html "enum gix::worktree::IndexPersistedOrInMemory"), [Error](https://docs.rs/gix/latest/gix/repository/index_or_load_from_head_or_empty/enum.Error.html "enum gix::repository::index_or_load_from_head_or_empty::Error")\>

Available on **crate feature `index`** only.

Open the persisted worktree index or generate it from the current `HEAD^{tree}` to live in-memory only, or resort to an empty index if `HEAD` is unborn.

Use this method to get an index in any repository, even bare ones that don’t have one naturally, or those that are in a state where `HEAD` is invalid or points to an unborn reference.

[Source](https://docs.rs/gix/latest/src/gix/repository/index.rs.html#146-156)

#### pub fn [index\_from\_tree](#method.index_from_tree)(&self, tree: &[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid")) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[File](https://docs.rs/gix/latest/gix/index/struct.File.html "struct gix::index::File"), [Error](https://docs.rs/gix/latest/gix/repository/index_from_tree/enum.Error.html "enum gix::repository::index_from_tree::Error")\>

Available on **crate feature `index`** only.

Create new index-file, which would live at the correct location, in memory from the given `tree`.

Note that this is an expensive operation as it requires recursively traversing the entire tree to unpack it into the index.

[Source](https://docs.rs/gix/latest/src/gix/repository/init.rs.html#3-37)[§](#impl-Repository-15)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/init.rs.html#34-36)

#### pub fn [into\_sync](#method.into_sync)(self) -> [ThreadSafeRepository](https://docs.rs/gix/latest/gix/struct.ThreadSafeRepository.html "struct gix::ThreadSafeRepository")

Convert this instance into a [`ThreadSafeRepository`](https://docs.rs/gix/latest/gix/struct.ThreadSafeRepository.html "struct gix::ThreadSafeRepository") by dropping all thread-local data.

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#7-111)[§](#impl-Repository-16)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#11-13)

#### pub fn [git\_dir](#method.git_dir)(&self) -> &[Path](https://doc.rust-lang.org/nightly/std/path/struct.Path.html "struct std::path::Path")

Return the path to the repository itself, containing objects, references, configuration, and more.

Synonymous to [`path()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.path "method gix::Repository::path").

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#17-19)

#### pub fn [git\_dir\_trust](#method.git_dir_trust)(&self) -> [Trust](https://docs.rs/gix-sec/0.11.0/x86_64-unknown-linux-gnu/gix_sec/enum.Trust.html "enum gix_sec::Trust")

The trust we place in the git-dir, with lower amounts of trust causing access to configuration to be limited. Note that if the git-dir is trusted but the worktree is not, the result is that the git-dir is also less trusted.

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#25-30)

#### pub fn [current\_dir](#method.current_dir)(&self) -> &[Path](https://doc.rust-lang.org/nightly/std/path/struct.Path.html "struct std::path::Path")

Return the current working directory as present during the instantiation of this repository.

Note that this should be preferred over manually obtaining it as this may have been adjusted to deal with `core.precomposeUnicode`.

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#33-35)

#### pub fn [common\_dir](#method.common_dir)(&self) -> &[Path](https://doc.rust-lang.org/nightly/std/path/struct.Path.html "struct std::path::Path")

Returns the main git repository if this is a repository on a linked work-tree, or the `git_dir` itself.

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#38-40)

#### pub fn [index\_path](#method.index_path)(&self) -> [PathBuf](https://doc.rust-lang.org/nightly/std/path/struct.PathBuf.html "struct std::path::PathBuf")

Return the path to the worktree index file, which may or may not exist.

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#44-46)

#### pub fn [modules\_path](#method.modules_path)(&self) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[PathBuf](https://doc.rust-lang.org/nightly/std/path/struct.PathBuf.html "struct std::path::PathBuf")\>

Available on **crate feature `attributes`** only.

The path to the `.gitmodules` file in the worktree, if a worktree is available.

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#49-51)

#### pub fn [path](#method.path)(&self) -> &[Path](https://doc.rust-lang.org/nightly/std/path/struct.Path.html "struct std::path::Path")

The path to the `.git` directory itself, or equivalent if this is a bare repository.

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#56-58)

#### pub fn [work\_dir](#method.work_dir)(&self) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<&[Path](https://doc.rust-lang.org/nightly/std/path/struct.Path.html "struct std::path::Path")\>

👎Deprecated: Use `workdir()` instead

Return the work tree containing all checked out files, if there is one.

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#61-63)

#### pub fn [workdir](#method.workdir)(&self) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<&[Path](https://doc.rust-lang.org/nightly/std/path/struct.Path.html "struct std::path::Path")\>

Return the work tree containing all checked out files, if there is one.

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#67-73)

#### pub fn [workdir\_path](#method.workdir_path)(&self, rela\_path: impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[PathBuf](https://doc.rust-lang.org/nightly/std/path/struct.PathBuf.html "struct std::path::PathBuf")\>

Turn `rela_path` into a path qualified with the [`workdir()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.workdir "method gix::Repository::workdir") of this instance, if one is available.

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#77-79)

#### pub fn [install\_dir](#method.install_dir)(&self) -> [Result](https://doc.rust-lang.org/nightly/std/io/error/type.Result.html "type std::io::error::Result")<[PathBuf](https://doc.rust-lang.org/nightly/std/path/struct.PathBuf.html "struct std::path::PathBuf")\>

The directory of the binary path of the current process.

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#86-94)

#### pub fn [prefix](#method.prefix)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<&[Path](https://doc.rust-lang.org/nightly/std/path/struct.Path.html "struct std::path::Path")\>, [Error](https://docs.rs/gix/latest/gix/path/realpath/enum.Error.html "enum gix::path::realpath::Error")\>

Returns the relative path which is the components between the working tree and the current working dir (CWD). Note that it may be `None` if there is no work tree, or if CWD isn’t inside of the working tree directory.

Note that the CWD is obtained once upon instantiation of the repository.

[Source](https://docs.rs/gix/latest/src/gix/repository/location.rs.html#97-110)

#### pub fn [kind](#method.kind)(&self) -> [Kind](https://docs.rs/gix/latest/gix/repository/enum.Kind.html "enum gix::repository::Kind")

Return the kind of repository, either bare or one with a work tree.

[Source](https://docs.rs/gix/latest/src/gix/repository/mailmap.rs.html#3-85)[§](#impl-Repository-17)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/mailmap.rs.html#10-14)

#### pub fn [open\_mailmap](#method.open_mailmap)(&self) -> [Snapshot](https://docs.rs/gix/latest/gix/mailmap/struct.Snapshot.html "struct gix::mailmap::Snapshot")

Available on **crate feature `mailmap`** only.

Similar to [`open_mailmap_into()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.open_mailmap_into "method gix::Repository::open_mailmap_into"), but ignores all errors and returns at worst an empty mailmap, e.g. if there is no mailmap or if there were errors loading them.

This represents typical usage within git, which also works with what’s there without considering a populated mailmap a reason to abort an operation, considering it optional.

[Source](https://docs.rs/gix/latest/src/gix/repository/mailmap.rs.html#26-84)

#### pub fn [open\_mailmap\_into](#method.open_mailmap_into)(&self, target: &mut [Snapshot](https://docs.rs/gix/latest/gix/mailmap/struct.Snapshot.html "struct gix::mailmap::Snapshot")) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[()](https://doc.rust-lang.org/nightly/std/primitive.unit.html), [Error](https://docs.rs/gix/latest/gix/mailmap/load/enum.Error.html "enum gix::mailmap::load::Error")\>

Available on **crate feature `mailmap`** only.

Try to merge mailmaps from the following locations into `target`:

*   read the `.mailmap` file without following symlinks from the working tree, if present
*   OR read `HEAD:.mailmap` if this repository is bare (i.e. has no working tree), if the `mailmap.blob` is not set.
*   read the mailmap as configured in `mailmap.blob`, if set.
*   read the file as configured by `mailmap.file`, following symlinks, if set.

Only the first error will be reported, and as many source mailmaps will be merged into `target` as possible. Parsing errors will be ignored.

[Source](https://docs.rs/gix/latest/src/gix/repository/merge.rs.html#17-307)[§](#impl-Repository-18)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Merge-utilities

[Source](https://docs.rs/gix/latest/src/gix/repository/merge.rs.html#24-64)

#### pub fn [merge\_resource\_cache](#method.merge_resource_cache)( &self, worktree\_roots: [WorktreeRoots](https://docs.rs/gix/latest/gix/merge/blob/pipeline/struct.WorktreeRoots.html "struct gix::merge::blob::pipeline::WorktreeRoots"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Platform](https://docs.rs/gix/latest/gix/merge/blob/struct.Platform.html "struct gix::merge::blob::Platform"), [Error](https://docs.rs/gix/latest/gix/repository/merge_resource_cache/enum.Error.html "enum gix::repository::merge_resource_cache::Error")\>

Available on **crate feature `merge`** only.

Create a resource cache that can hold the three resources needed for a three-way merge. `worktree_roots` determines which side of the merge is read from the worktree, or from which worktree.

The platform can be used to set up resources and finally perform a merge among blobs.

Note that the current index is used for attribute queries.

[Source](https://docs.rs/gix/latest/src/gix/repository/merge.rs.html#68-90)

#### pub fn [blob\_merge\_options](#method.blob_merge_options)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Options](https://docs.rs/gix/latest/gix/merge/blob/platform/merge/struct.Options.html "struct gix::merge::blob::platform::merge::Options"), [Error](https://docs.rs/gix/latest/gix/repository/blob_merge_options/enum.Error.html "enum gix::repository::blob_merge_options::Error")\>

Available on **crate feature `merge`** only.

Return options for use with [`gix_merge::blob::PlatformRef::merge()`](https://docs.rs/gix/latest/gix/merge/blob/struct.PlatformRef.html#method.merge "method gix::merge::blob::PlatformRef::merge"), accessible through [merge\_resource\_cache()](https://docs.rs/gix/latest/gix/struct.Repository.html#method.merge_resource_cache "method gix::Repository::merge_resource_cache").

[Source](https://docs.rs/gix/latest/src/gix/repository/merge.rs.html#93-117)

#### pub fn [tree\_merge\_options](#method.tree_merge_options)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Options](https://docs.rs/gix/latest/gix/merge/tree/struct.Options.html "struct gix::merge::tree::Options"), [Error](https://docs.rs/gix/latest/gix/repository/tree_merge_options/enum.Error.html "enum gix::repository::tree_merge_options::Error")\>

Available on **crate feature `merge`** only.

Read all relevant configuration options to instantiate options for use in [`merge_trees()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.merge_trees "method gix::Repository::merge_trees").

[Source](https://docs.rs/gix/latest/src/gix/repository/merge.rs.html#135-172)

#### pub fn [merge\_trees](#method.merge_trees)( &self, ancestor\_tree: impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid")\>, our\_tree: impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid")\>, their\_tree: impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid")\>, labels: [Labels](https://docs.rs/gix/latest/gix/merge/blob/builtin_driver/text/struct.Labels.html "struct gix::merge::blob::builtin_driver::text::Labels")<'\_>, options: [Options](https://docs.rs/gix/latest/gix/merge/tree/struct.Options.html "struct gix::merge::tree::Options"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Outcome](https://docs.rs/gix/latest/gix/merge/tree/struct.Outcome.html "struct gix::merge::tree::Outcome")<'\_>, [Error](https://docs.rs/gix/latest/gix/repository/merge_trees/enum.Error.html "enum gix::repository::merge_trees::Error")\>

Available on **crate feature `merge`** only.

Merge `our_tree` and `their_tree` together, assuming they have the same `ancestor_tree`, to yield a new tree which is provided as [tree editor](https://docs.rs/gix/latest/gix/object/tree/struct.Editor.html "struct gix::object::tree::Editor") to inspect and finalize results at will. No change to the worktree or index is made, but objects may be written to the object database as merge results are stored. If these changes should not be observable outside of this instance, consider [enabling object memory](https://docs.rs/gix/latest/gix/struct.Repository.html#method.with_object_memory "method gix::Repository::with_object_memory").

Note that `ancestor_tree` can be the [empty tree hash](https://docs.rs/gix/latest/gix/enum.ObjectId.html#method.empty_tree "associated function gix::ObjectId::empty_tree") to indicate no common ancestry.

`labels` are typically chosen to identify the refs or names for `our_tree` and `their_tree` and `ancestor_tree` respectively.

`options` should be initialized with [`tree_merge_options()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.tree_merge_options "method gix::Repository::tree_merge_options").

###### [§](#performance-2)Performance

It’s highly recommended to [set an object cache](https://docs.rs/gix/latest/gix/struct.Repository.html#method.compute_object_cache_size_for_tree_diffs "method gix::Repository::compute_object_cache_size_for_tree_diffs") to avoid extracting the same object multiple times.

[Source](https://docs.rs/gix/latest/src/gix/repository/merge.rs.html#190-239)

#### pub fn [merge\_commits](#method.merge_commits)( &self, our\_commit: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, their\_commit: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, labels: [Labels](https://docs.rs/gix/latest/gix/merge/blob/builtin_driver/text/struct.Labels.html "struct gix::merge::blob::builtin_driver::text::Labels")<'\_>, options: [Options](https://docs.rs/gix/latest/gix/merge/commit/struct.Options.html "struct gix::merge::commit::Options"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Outcome](https://docs.rs/gix/latest/gix/merge/commit/struct.Outcome.html "struct gix::merge::commit::Outcome")<'\_>, [Error](https://docs.rs/gix/latest/gix/repository/merge_commits/enum.Error.html "enum gix::repository::merge_commits::Error")\>

Available on **crate feature `merge`** only.

Merge `our_commit` and `their_commit` together to yield a new tree which is provided as [tree editor](https://docs.rs/gix/latest/gix/object/tree/struct.Editor.html "struct gix::object::tree::Editor") to inspect and finalize results at will. The merge-base will be determined automatically between both commits, along with special handling in case there are multiple merge-bases. No change to the worktree or index is made, but objects may be written to the object database as merge results are stored. If these changes should not be observable outside of this instance, consider [enabling object memory](https://docs.rs/gix/latest/gix/struct.Repository.html#method.with_object_memory "method gix::Repository::with_object_memory").

`labels` are typically chosen to identify the refs or names for `our_commit` and `their_commit`, with the ancestor being set automatically as part of the merge-base handling.

`options` should be initialized with [`Repository::tree_merge_options().into()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.tree_merge_options "method gix::Repository::tree_merge_options").

###### [§](#performance-3)Performance

It’s highly recommended to [set an object cache](https://docs.rs/gix/latest/gix/struct.Repository.html#method.compute_object_cache_size_for_tree_diffs "method gix::Repository::compute_object_cache_size_for_tree_diffs") to avoid extracting the same object multiple times.

[Source](https://docs.rs/gix/latest/src/gix/repository/merge.rs.html#250-258)

#### pub fn [virtual\_merge\_base](#method.virtual_merge_base)( &self, merge\_bases: impl [IntoIterator](https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")<Item = impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>>, options: [Options](https://docs.rs/gix/latest/gix/merge/tree/struct.Options.html "struct gix::merge::tree::Options"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Outcome](https://docs.rs/gix/latest/gix/merge/virtual_merge_base/struct.Outcome.html "struct gix::merge::virtual_merge_base::Outcome")<'\_>, [Error](https://docs.rs/gix/latest/gix/repository/virtual_merge_base/enum.Error.html "enum gix::repository::virtual_merge_base::Error")\>

Available on **crate feature `merge`** only.

Create a single virtual merge-base by merging all `merge_bases` into one. If the list is empty, an error will be returned as the histories are then unrelated. If there is only one commit in the list, it is returned directly with this case clearly marked in the outcome.

Note that most of `options` are overwritten to match the requirements of a merge-base merge, but they can be useful to control the diff algorithm or rewrite tracking, for example.

This method is useful in conjunction with [`Self::merge_trees()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.merge_trees "method gix::Repository::merge_trees"), as the ancestor tree can be produced here.

[Source](https://docs.rs/gix/latest/src/gix/repository/merge.rs.html#262-306)

#### pub fn [virtual\_merge\_base\_with\_graph](#method.virtual_merge_base_with_graph)( &self, merge\_bases: impl [IntoIterator](https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")<Item = impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>>, graph: &mut [Graph](https://docs.rs/gix-revwalk/0.20.1/x86_64-unknown-linux-gnu/gix_revwalk/struct.Graph.html "struct gix_revwalk::Graph")<'\_, '\_, [Commit](https://docs.rs/gix-revwalk/0.20.1/x86_64-unknown-linux-gnu/gix_revwalk/graph/struct.Commit.html "struct gix_revwalk::graph::Commit")<[Flags](https://docs.rs/gix-revision/0.34.1/x86_64-unknown-linux-gnu/gix_revision/merge_base/struct.Flags.html "struct gix_revision::merge_base::Flags")\>>, options: [Options](https://docs.rs/gix/latest/gix/merge/tree/struct.Options.html "struct gix::merge::tree::Options"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Outcome](https://docs.rs/gix/latest/gix/merge/virtual_merge_base/struct.Outcome.html "struct gix::merge::virtual_merge_base::Outcome")<'\_>, [Error](https://docs.rs/gix/latest/gix/repository/virtual_merge_base_with_graph/enum.Error.html "enum gix::repository::virtual_merge_base_with_graph::Error")\>

Available on **crate feature `merge`** only.

Like [`Self::virtual_merge_base()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.virtual_merge_base "method gix::Repository::virtual_merge_base"), but also allows to reuse a `graph` for faster merge-base calculation, particularly if `graph` was used to find the `merge_bases`.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#17-29)[§](#impl-Repository-19)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Tree editing

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#22-28)

#### pub fn [edit\_tree](#method.edit_tree)(&self, id: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Editor](https://docs.rs/gix/latest/gix/object/tree/struct.Editor.html "struct gix::object::tree::Editor")<'\_>, [Error](https://docs.rs/gix/latest/gix/repository/edit_tree/enum.Error.html "enum gix::repository::edit_tree::Error")\>

Available on **crate feature `tree-editor`** only.

Return an editor for adjusting the tree at `id`.

This can be the [empty tree id](https://docs.rs/gix/latest/gix/enum.ObjectId.html#method.empty_tree "associated function gix::ObjectId::empty_tree") to build a tree from scratch.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#32-157)[§](#impl-Repository-20)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Find objects of various kins

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#42-55)

#### pub fn [find\_object](#method.find_object)(&self, id: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Object](https://docs.rs/gix/latest/gix/struct.Object.html "struct gix::Object")<'\_>, [Error](https://docs.rs/gix/latest/gix/object/find/existing/type.Error.html "type gix::object::find::existing::Error")\>

Find the object with `id` in the object database or return an error if it could not be found.

There are various legitimate reasons for an object to not be present, which is why [`try_find_object(…)`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.try_find_object "method gix::Repository::try_find_object") might be preferable instead.

##### [§](#performance-note)Performance Note

In order to get the kind of the object, is must be fully decoded from storage if it is packed with deltas. Loose object could be partially decoded, even though that’s not implemented.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#58-63)

#### pub fn [find\_commit](#method.find_commit)(&self, id: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Commit](https://docs.rs/gix/latest/gix/struct.Commit.html "struct gix::Commit")<'\_>, [Error](https://docs.rs/gix/latest/gix/object/find/existing/with_conversion/enum.Error.html "enum gix::object::find::existing::with_conversion::Error")\>

Find a commit with `id` or fail if there was no object or the object wasn’t a commit.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#66-71)

#### pub fn [find\_tree](#method.find_tree)(&self, id: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Tree](https://docs.rs/gix/latest/gix/struct.Tree.html "struct gix::Tree")<'\_>, [Error](https://docs.rs/gix/latest/gix/object/find/existing/with_conversion/enum.Error.html "enum gix::object::find::existing::with_conversion::Error")\>

Find a tree with `id` or fail if there was no object or the object wasn’t a tree.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#74-76)

#### pub fn [find\_tag](#method.find_tag)(&self, id: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Tag](https://docs.rs/gix/latest/gix/struct.Tag.html "struct gix::Tag")<'\_>, [Error](https://docs.rs/gix/latest/gix/object/find/existing/with_conversion/enum.Error.html "enum gix::object::find::existing::with_conversion::Error")\>

Find an annotated tag with `id` or fail if there was no object or the object wasn’t a tag.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#79-84)

#### pub fn [find\_blob](#method.find_blob)(&self, id: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Blob](https://docs.rs/gix/latest/gix/struct.Blob.html "struct gix::Blob")<'\_>, [Error](https://docs.rs/gix/latest/gix/object/find/existing/with_conversion/enum.Error.html "enum gix::object::find::existing::with_conversion::Error")\>

Find a blob with `id` or fail if there was no object or the object wasn’t a blob.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#90-99)

#### pub fn [find\_header](#method.find_header)(&self, id: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Header](https://docs.rs/gix-odb/0.69.1/x86_64-unknown-linux-gnu/gix_odb/find/enum.Header.html "enum gix_odb::find::Header"), [Error](https://docs.rs/gix/latest/gix/object/find/existing/type.Error.html "type gix::object::find::existing::Error")\>

Obtain information about an object without fully decoding it, or fail if the object doesn’t exist.

Note that despite being cheaper than [`Self::find_object()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.find_object "method gix::Repository::find_object"), there is still some effort traversing delta-chains.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#110-117)

#### pub fn [has\_object](#method.has_object)(&self, id: impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid")\>) -> [bool](https://doc.rust-lang.org/nightly/std/primitive.bool.html)

Return `true` if `id` exists in the object database.

##### [§](#performance-4)Performance

This method can be slow if the underlying [object database](https://docs.rs/gix/latest/gix/struct.Repository.html#structfield.objects "field gix::Repository::objects") has an unsuitable [RefreshMode](https://docs.rs/gix-odb/0.69.1/x86_64-unknown-linux-gnu/gix_odb/store_impls/dynamic/enum.RefreshMode.html "enum gix_odb::store_impls::dynamic::RefreshMode") and `id` is not likely to exist. Use [`repo.objects.refresh_never()`](https://docs.rs/gix-odb/0.69.1/x86_64-unknown-linux-gnu/gix_odb/store_impls/dynamic/struct.Handle.html#method.refresh_never "method gix_odb::store_impls::dynamic::Handle::refresh_never") to avoid expensive IO-bound refreshes if an object wasn’t found.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#122-134)

#### pub fn [try\_find\_header](#method.try_find_header)( &self, id: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Header](https://docs.rs/gix-odb/0.69.1/x86_64-unknown-linux-gnu/gix_odb/find/enum.Header.html "enum gix_odb::find::Header")\>, [Error](https://docs.rs/gix/latest/gix/object/find/struct.Error.html "struct gix::object::find::Error")\>

Obtain information about an object without fully decoding it, or `None` if the object doesn’t exist.

Note that despite being cheaper than [`Self::try_find_object()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.try_find_object "method gix::Repository::try_find_object"), there is still some effort traversing delta-chains.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#137-156)

#### pub fn [try\_find\_object](#method.try_find_object)( &self, id: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Object](https://docs.rs/gix/latest/gix/struct.Object.html "struct gix::Object")<'\_>>, [Error](https://docs.rs/gix/latest/gix/object/find/struct.Error.html "struct gix::object::find::Error")\>

Try to find the object with `id` or return `None` if it wasn’t found.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#160-230)[§](#impl-Repository-21)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Write objects of any type.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#165-172)

#### pub fn [write\_object](#method.write_object)(&self, object: impl [WriteTo](https://docs.rs/gix/latest/gix/diff/object/trait.WriteTo.html "trait gix::diff::object::WriteTo")) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'\_>, [Error](https://docs.rs/gix/latest/gix/object/write/struct.Error.html "struct gix::object::write::Error")\>

Write the given object into the object database and return its object id.

Note that we hash the object in memory to avoid storing objects that are already present. That way, we avoid writing duplicate objects using slow disks that will eventually have to be garbage collected.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#191-202)

#### pub fn [write\_blob](#method.write_blob)(&self, bytes: impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<\[[u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html)\]>) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'\_>, [Error](https://docs.rs/gix/latest/gix/object/write/struct.Error.html "struct gix::object::write::Error")\>

Write a blob from the given `bytes`.

We avoid writing duplicate objects to slow disks that will eventually have to be garbage collected by pre-hashing the data, and checking if the object is already present.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#210-216)

#### pub fn [write\_blob\_stream](#method.write_blob_stream)(&self, bytes: impl [Read](https://doc.rust-lang.org/nightly/std/io/trait.Read.html "trait std::io::Read")) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'\_>, [Error](https://docs.rs/gix/latest/gix/object/write/struct.Error.html "struct gix::object::write::Error")\>

Write a blob from the given `Read` implementation.

Note that we hash the object in memory to avoid storing objects that are already present. That way, we avoid writing duplicate objects using slow disks that will eventually have to be garbage collected.

If that is prohibitive, use the object database directly.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#233-400)[§](#impl-Repository-22)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Create commits and tags

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#239-258)

#### pub fn [tag](#method.tag)( &self, name: impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[str](https://doc.rust-lang.org/nightly/std/primitive.str.html)\>, target: impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid")\>, target\_kind: [Kind](https://docs.rs/gix/latest/gix/object/enum.Kind.html "enum gix::object::Kind"), tagger: [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[SignatureRef](https://docs.rs/gix-actor/0.35.1/x86_64-unknown-linux-gnu/gix_actor/struct.SignatureRef.html "struct gix_actor::SignatureRef")<'\_>>, message: impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[str](https://doc.rust-lang.org/nightly/std/primitive.str.html)\>, constraint: [PreviousValue](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/transaction/enum.PreviousValue.html "enum gix_ref::transaction::PreviousValue"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Reference](https://docs.rs/gix/latest/gix/struct.Reference.html "struct gix::Reference")<'\_>, [Error](https://docs.rs/gix/latest/gix/tag/enum.Error.html "enum gix::tag::Error")\>

Create a tag reference named `name` (without `refs/tags/` prefix) pointing to a newly created tag object which in turn points to `target` and return the newly created reference.

It will be created with `constraint` which is most commonly to [only create it](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/transaction/enum.PreviousValue.html#variant.MustNotExist "variant gix_ref::transaction::PreviousValue::MustNotExist") or to [force overwriting a possibly existing tag](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/transaction/enum.PreviousValue.html#variant.Any "variant gix_ref::transaction::PreviousValue::Any").

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#263-284)

#### pub fn [commit\_as](#method.commit_as)<'a, 'c, Name, E>( &self, committer: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[SignatureRef](https://docs.rs/gix-actor/0.35.1/x86_64-unknown-linux-gnu/gix_actor/struct.SignatureRef.html "struct gix_actor::SignatureRef")<'c>>, author: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[SignatureRef](https://docs.rs/gix-actor/0.35.1/x86_64-unknown-linux-gnu/gix_actor/struct.SignatureRef.html "struct gix_actor::SignatureRef")<'a>>, reference: Name, message: impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[str](https://doc.rust-lang.org/nightly/std/primitive.str.html)\>, tree: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, parents: impl [IntoIterator](https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")<Item = impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'\_>, [Error](https://docs.rs/gix/latest/gix/commit/enum.Error.html "enum gix::commit::Error")\>

where Name: [TryInto](https://doc.rust-lang.org/nightly/core/convert/trait.TryInto.html "trait core::convert::TryInto")<[FullName](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.FullName.html "struct gix_ref::FullName"), Error = E>, [Error](https://docs.rs/gix/latest/gix/commit/enum.Error.html "enum gix::commit::Error"): [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<E>,

Similar to [`commit(…)`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.commit "method gix::Repository::commit"), but allows to create the commit with `committer` and `author` specified.

This forces setting the commit time and author time by hand. Note that typically, committer and author are the same.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#363-377)

#### pub fn [commit](#method.commit)<Name, E>( &self, reference: Name, message: impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[str](https://doc.rust-lang.org/nightly/std/primitive.str.html)\>, tree: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, parents: impl [IntoIterator](https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")<Item = impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'\_>, [Error](https://docs.rs/gix/latest/gix/commit/enum.Error.html "enum gix::commit::Error")\>

where Name: [TryInto](https://doc.rust-lang.org/nightly/core/convert/trait.TryInto.html "trait core::convert::TryInto")<[FullName](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.FullName.html "struct gix_ref::FullName"), Error = E>, [Error](https://docs.rs/gix/latest/gix/commit/enum.Error.html "enum gix::commit::Error"): [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<E>,

Create a new commit object with `message` referring to `tree` with `parents`, and point `reference` to it. The commit is written without message encoding field, which can be assumed to be UTF-8. `author` and `committer` fields are pre-set from the configuration, which can be altered [temporarily](https://docs.rs/gix/latest/gix/struct.Repository.html#method.config_snapshot_mut "method gix::Repository::config_snapshot_mut") before the call if required.

`reference` will be created if it doesn’t exist, and can be `"HEAD"` to automatically write-through to the symbolic reference that `HEAD` points to if it is not detached. For this reason, detached head states cannot be created unless the `HEAD` is detached already. The reflog will be written as canonical git would do, like `<operation> (<detail>): <summary>`.

The first parent id in `parents` is expected to be the current target of `reference` and the operation will fail if it is not. If there is no parent, the `reference` is expected to not exist yet.

The method fails immediately if a `reference` lock can’t be acquired.

###### [§](#writing-a-commit-without-reference-update)Writing a commit without `reference` update

If the reference shouldn’t be updated, use [`Self::write_object()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.write_object "method gix::Repository::write_object") along with a newly created [`crate::objs::Object`](https://docs.rs/gix/latest/gix/diff/object/enum.Object.html "enum gix::diff::object::Object") whose fields can be fully defined.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#383-387)

#### pub fn [empty\_tree](#method.empty_tree)(&self) -> [Tree](https://docs.rs/gix/latest/gix/struct.Tree.html "struct gix::Tree")<'\_>

Return an empty tree object, suitable for [getting changes](https://docs.rs/gix/latest/gix/struct.Tree.html#method.changes "method gix::Tree::changes").

Note that the returned object is special and doesn’t necessarily physically exist in the object database. This means that this object can be used in an uninitialized, empty repository which would report to have no objects at all.

[Source](https://docs.rs/gix/latest/src/gix/repository/object.rs.html#393-399)

#### pub fn [empty\_blob](#method.empty_blob)(&self) -> [Blob](https://docs.rs/gix/latest/gix/struct.Blob.html "struct gix::Blob")<'\_>

Return an empty blob object.

Note that the returned object is special and doesn’t necessarily physically exist in the object database. This means that this object can be used in an uninitialized, empty repository which would report to have no objects at all.

[Source](https://docs.rs/gix/latest/src/gix/repository/pathspec.rs.html#5-58)[§](#impl-Repository-23)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/pathspec.rs.html#17-30)

#### pub fn [pathspec](#method.pathspec)( &self, empty\_patterns\_match\_prefix: [bool](https://doc.rust-lang.org/nightly/std/primitive.bool.html), patterns: impl [IntoIterator](https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")<Item = impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>>, inherit\_ignore\_case: [bool](https://doc.rust-lang.org/nightly/std/primitive.bool.html), index: &[State](https://docs.rs/gix/latest/gix/index/struct.State.html "struct gix::index::State"), attributes\_source: [Source](https://docs.rs/gix/latest/gix/worktree/stack/state/attributes/enum.Source.html "enum gix::worktree::stack::state::attributes::Source"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Pathspec](https://docs.rs/gix/latest/gix/struct.Pathspec.html "struct gix::Pathspec")<'\_>, [Error](https://docs.rs/gix/latest/gix/pathspec/init/enum.Error.html "enum gix::pathspec::init::Error")\>

Available on **crate feature `attributes`** only.

Create a new pathspec abstraction that allows to conduct searches using `patterns`. `inherit_ignore_case` should be `true` if `patterns` will match against files on disk, or `false` otherwise, for more natural matching (but also note that `git` does not do that). `index` may be needed to load attributes which is required only if `patterns` refer to attributes via `:(attr:…)` syntax. In the same vein, `attributes_source` affects where `.gitattributes` files are read from if pathspecs need to match against attributes. If `empty_patterns_match_prefix` is `true`, then even empty patterns will match only what’s inside of the prefix. Otherwise they will match everything.

It will be initialized exactly how it would, and attribute matching will be conducted by reading the worktree first if available. If that is not desirable, consider calling [`Pathspec::new()`](https://docs.rs/gix/latest/gix/struct.Pathspec.html#method.new "associated function gix::Pathspec::new") directly.

[Source](https://docs.rs/gix/latest/src/gix/repository/pathspec.rs.html#36-38)

#### pub fn [pathspec\_defaults](#method.pathspec_defaults)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Defaults](https://docs.rs/gix/latest/gix/pathspec/struct.Defaults.html "struct gix::pathspec::Defaults"), [Error](https://docs.rs/gix/latest/gix/pathspec/defaults/from_environment/enum.Error.html "enum gix::pathspec::defaults::from_environment::Error")\>

Available on **crate feature `attributes`** only.

Return default settings that are required when [parsing pathspecs](https://docs.rs/gix/latest/gix/pathspec/fn.parse.html "fn gix::pathspec::parse") by hand.

These are stemming from environment variables which have been converted to [config settings](https://docs.rs/gix/latest/gix/config/tree/gitoxide/struct.Pathspec.html "struct gix::config::tree::gitoxide::Pathspec"), which now serve as authority for configuration.

[Source](https://docs.rs/gix/latest/src/gix/repository/pathspec.rs.html#42-57)

#### pub fn [pathspec\_defaults\_inherit\_ignore\_case](#method.pathspec_defaults_inherit_ignore_case)( &self, inherit\_ignore\_case: [bool](https://doc.rust-lang.org/nightly/std/primitive.bool.html), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Defaults](https://docs.rs/gix/latest/gix/pathspec/struct.Defaults.html "struct gix::pathspec::Defaults"), [Error](https://docs.rs/gix/latest/gix/repository/pathspec_defaults_ignore_case/enum.Error.html "enum gix::repository::pathspec_defaults_ignore_case::Error")\>

Available on **crate feature `attributes`** only.

Similar to [Self::pathspec\_defaults()](https://docs.rs/gix/latest/gix/struct.Repository.html#method.pathspec_defaults "method gix::Repository::pathspec_defaults"), but will automatically configure the returned defaults to match case-insensitively if the underlying filesystem is also configured to be case-insensitive according to `core.ignoreCase`, and `inherit_ignore_case` is `true`.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#10-306)[§](#impl-Repository-24)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Obtain and alter references comfortably

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#15-41)

#### pub fn [tag\_reference](#method.tag_reference)( &self, name: impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[str](https://doc.rust-lang.org/nightly/std/primitive.str.html)\>, target: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, constraint: [PreviousValue](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/transaction/enum.PreviousValue.html "enum gix_ref::transaction::PreviousValue"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Reference](https://docs.rs/gix/latest/gix/struct.Reference.html "struct gix::Reference")<'\_>, [Error](https://docs.rs/gix/latest/gix/reference/edit/enum.Error.html "enum gix::reference::edit::Error")\>

Create a lightweight tag with given `name` (and without `refs/tags/` prefix) pointing to the given `target`, and return it as reference.

It will be created with `constraint` which is most commonly to [only create it](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/transaction/enum.PreviousValue.html#variant.MustNotExist "variant gix_ref::transaction::PreviousValue::MustNotExist") or to [force overwriting a possibly existing tag](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/transaction/enum.PreviousValue.html#variant.Any "variant gix_ref::transaction::PreviousValue::Any").

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#46-48)

#### pub fn [namespace](#method.namespace)(&self) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<&[Namespace](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.Namespace.html "struct gix_ref::Namespace")\>

Returns the currently set namespace for references, or `None` if it is not set.

Namespaces allow to partition references, and is configured per `Easy`.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#51-53)

#### pub fn [clear\_namespace](#method.clear_namespace)(&mut self) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Namespace](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.Namespace.html "struct gix_ref::Namespace")\>

Remove the currently set reference namespace and return it, affecting only this `Easy`.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#58-68)

#### pub fn [set\_namespace](#method.set_namespace)<'a, Name, E>( &mut self, namespace: Name, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Namespace](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.Namespace.html "struct gix_ref::Namespace")\>, [Error](https://docs.rs/gix/latest/gix/index/validate/reference/name/enum.Error.html "enum gix::index::validate::reference::name::Error")\>

where Name: [TryInto](https://doc.rust-lang.org/nightly/core/convert/trait.TryInto.html "trait core::convert::TryInto")<&'a [PartialNameRef](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.PartialNameRef.html "struct gix_ref::PartialNameRef"), Error = E>, [Error](https://docs.rs/gix/latest/gix/index/validate/reference/name/enum.Error.html "enum gix::index::validate::reference::name::Error"): [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<E>,

Set the reference namespace to the given value, like `"foo"` or `"foo/bar"`.

Note that this value is shared across all `Easy…` instances as the value is stored in the shared `Repository`.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#75-92)

#### pub fn [reference](#method.reference)<Name, E>( &self, name: Name, target: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, constraint: [PreviousValue](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/transaction/enum.PreviousValue.html "enum gix_ref::transaction::PreviousValue"), log\_message: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[BString](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BString.html "struct gix::diff::object::bstr::BString")\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Reference](https://docs.rs/gix/latest/gix/struct.Reference.html "struct gix::Reference")<'\_>, [Error](https://docs.rs/gix/latest/gix/reference/edit/enum.Error.html "enum gix::reference::edit::Error")\>

where Name: [TryInto](https://doc.rust-lang.org/nightly/core/convert/trait.TryInto.html "trait core::convert::TryInto")<[FullName](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.FullName.html "struct gix_ref::FullName"), Error = E>, [Error](https://docs.rs/gix/latest/gix/index/validate/reference/name/enum.Error.html "enum gix::index::validate::reference::name::Error"): [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<E>,

Create a new reference with `name`, like `refs/heads/branch`, pointing to `target`, adhering to `constraint` during creation and writing `log_message` into the reflog. Note that a ref-log will be written even if `log_message` is empty.

The newly created Reference is returned.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#132-134)

#### pub fn [edit\_reference](#method.edit_reference)(&self, edit: [RefEdit](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/transaction/struct.RefEdit.html "struct gix_ref::transaction::RefEdit")) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[RefEdit](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/transaction/struct.RefEdit.html "struct gix_ref::transaction::RefEdit")\>, [Error](https://docs.rs/gix/latest/gix/reference/edit/enum.Error.html "enum gix::reference::edit::Error")\>

Edit a single reference as described in `edit`, and write reference logs as `log_committer`.

One or more `RefEdit`s are returned - symbolic reference splits can cause more edits to be performed. All edits have the previous reference values set to the ones encountered at rest after acquiring the respective reference’s lock.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#143-148)

#### pub fn [edit\_references](#method.edit_references)( &self, edits: impl [IntoIterator](https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")<Item = [RefEdit](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/transaction/struct.RefEdit.html "struct gix_ref::transaction::RefEdit")\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[RefEdit](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/transaction/struct.RefEdit.html "struct gix_ref::transaction::RefEdit")\>, [Error](https://docs.rs/gix/latest/gix/reference/edit/enum.Error.html "enum gix::reference::edit::Error")\>

Edit one or more references as described by their `edits`. Note that one can set the committer name for use in the ref-log by temporarily [overriding the git-config](https://docs.rs/gix/latest/gix/struct.Repository.html#method.config_snapshot_mut "method gix::Repository::config_snapshot_mut"), or use [`edit_references_as(committer)`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.edit_references_as "method gix::Repository::edit_references_as") for convenience.

Returns all reference edits, which might be more than where provided due the splitting of symbolic references, and whose previous (_old_) values are the ones seen on in storage after the reference was locked.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#153-164)

#### pub fn [edit\_references\_as](#method.edit_references_as)( &self, edits: impl [IntoIterator](https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")<Item = [RefEdit](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/transaction/struct.RefEdit.html "struct gix_ref::transaction::RefEdit")\>, committer: [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[SignatureRef](https://docs.rs/gix-actor/0.35.1/x86_64-unknown-linux-gnu/gix_actor/struct.SignatureRef.html "struct gix_actor::SignatureRef")<'\_>>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[RefEdit](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/transaction/struct.RefEdit.html "struct gix_ref::transaction::RefEdit")\>, [Error](https://docs.rs/gix/latest/gix/reference/edit/enum.Error.html "enum gix::reference::edit::Error")\>

A way to apply reference `edits` similar to [edit\_references(…)](https://docs.rs/gix/latest/gix/struct.Repository.html#method.edit_references "method gix::Repository::edit_references"), but set a specific `commiter` for use in the reflog. It can be `None` if it’s the purpose `edits` are configured to not update the reference log, or cause a failure otherwise.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#169-183)

#### pub fn [head](#method.head)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Head](https://docs.rs/gix/latest/gix/struct.Head.html "struct gix::Head")<'\_>, [Error](https://docs.rs/gix/latest/gix/reference/find/existing/enum.Error.html "enum gix::reference::find::existing::Error")\>

Return the repository head, an abstraction to help dealing with the `HEAD` reference.

The `HEAD` reference can be in various states, for more information, the documentation of [`Head`](https://docs.rs/gix/latest/gix/struct.Head.html "struct gix::Head").

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#193-195)

#### pub fn [head\_id](#method.head_id)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'\_>, [Error](https://docs.rs/gix/latest/gix/reference/head_id/enum.Error.html "enum gix::reference::head_id::Error")\>

Resolve the `HEAD` reference, follow and peel its target and obtain its object id, following symbolic references and tags until a commit is found.

Note that this may fail for various reasons, most notably because the repository is freshly initialized and doesn’t have any commits yet.

Also note that the returned id is likely to point to a commit, but could also point to a tree or blob. It won’t, however, point to a tag as these are always peeled.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#201-203)

#### pub fn [head\_name](#method.head_name)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[FullName](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.FullName.html "struct gix_ref::FullName")\>, [Error](https://docs.rs/gix/latest/gix/reference/find/existing/enum.Error.html "enum gix::reference::find::existing::Error")\>

Return the name to the symbolic reference `HEAD` points to, or `None` if the head is detached.

The difference to [`head_ref()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.head_ref "method gix::Repository::head_ref") is that the latter requires the reference to exist, whereas here we merely return a the name of the possibly unborn reference.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#206-208)

#### pub fn [head\_ref](#method.head_ref)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Reference](https://docs.rs/gix/latest/gix/struct.Reference.html "struct gix::Reference")<'\_>>, [Error](https://docs.rs/gix/latest/gix/reference/find/existing/enum.Error.html "enum gix::reference::find::existing::Error")\>

Return the reference that `HEAD` points to, or `None` if the head is detached or unborn.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#216-218)

#### pub fn [head\_commit](#method.head_commit)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Commit](https://docs.rs/gix/latest/gix/struct.Commit.html "struct gix::Commit")<'\_>, [Error](https://docs.rs/gix/latest/gix/reference/head_commit/enum.Error.html "enum gix::reference::head_commit::Error")\>

Return the commit object the `HEAD` reference currently points to after peeling it fully, following symbolic references and tags until a commit is found.

Note that this may fail for various reasons, most notably because the repository is freshly initialized and doesn’t have any commits yet. It could also fail if the head does not point to a commit.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#226-228)

#### pub fn [head\_tree\_id](#method.head_tree_id)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'\_>, [Error](https://docs.rs/gix/latest/gix/reference/head_tree_id/enum.Error.html "enum gix::reference::head_tree_id::Error")\>

Return the tree id the `HEAD` reference currently points to after peeling it fully, following symbolic references and tags until a commit is found.

Note that this may fail for various reasons, most notably because the repository is freshly initialized and doesn’t have any commits yet. It could also fail if the head does not point to a commit.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#231-244)

#### pub fn [head\_tree\_id\_or\_empty](#method.head_tree_id_or_empty)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'\_>, [Error](https://docs.rs/gix/latest/gix/reference/head_tree_id/enum.Error.html "enum gix::reference::head_tree_id::Error")\>

Like [`Self::head_tree_id()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.head_tree_id "method gix::Repository::head_tree_id"), but will return an empty tree hash if the repository HEAD is unborn.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#252-254)

#### pub fn [head\_tree](#method.head_tree)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Tree](https://docs.rs/gix/latest/gix/struct.Tree.html "struct gix::Tree")<'\_>, [Error](https://docs.rs/gix/latest/gix/reference/head_tree/enum.Error.html "enum gix::reference::head_tree::Error")\>

Return the tree object the `HEAD^{tree}` reference currently points to after peeling it fully, following symbolic references and tags until a tree is found.

Note that this may fail for various reasons, most notably because the repository is freshly initialized and doesn’t have any commits yet. It could also fail if the head does not point to a tree, unlikely but possible.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#261-276)

#### pub fn [find\_reference](#method.find_reference)<'a, Name, E>( &self, name: Name, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Reference](https://docs.rs/gix/latest/gix/struct.Reference.html "struct gix::Reference")<'\_>, [Error](https://docs.rs/gix/latest/gix/reference/find/existing/enum.Error.html "enum gix::reference::find::existing::Error")\>

where Name: [TryInto](https://doc.rust-lang.org/nightly/core/convert/trait.TryInto.html "trait core::convert::TryInto")<&'a [PartialNameRef](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.PartialNameRef.html "struct gix_ref::PartialNameRef"), Error = E> + [Clone](https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html "trait core::clone::Clone"), [Error](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/store_impl/file/find/error/enum.Error.html "enum gix_ref::store_impl::file::find::error::Error"): [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<E>,

Find the reference with the given partial or full `name`, like `main`, `HEAD`, `heads/branch` or `origin/other`, or return an error if it wasn’t found.

Consider [`try_find_reference(…)`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.try_find_reference "method gix::Repository::try_find_reference") if the reference might not exist without that being considered an error.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#282-287)

#### pub fn [references](#method.references)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Platform](https://docs.rs/gix/latest/gix/reference/iter/struct.Platform.html "struct gix::reference::iter::Platform")<'\_>, [Error](https://docs.rs/gix/latest/gix/reference/iter/type.Error.html "type gix::reference::iter::Error")\>

Return a platform for iterating references.

Common kinds of iteration are [all](https://docs.rs/gix/latest/gix/reference/iter/struct.Platform.html#method.all "method gix::reference::iter::Platform::all") or [prefixed](https://docs.rs/gix/latest/gix/reference/iter/struct.Platform.html#method.prefixed "method gix::reference::iter::Platform::prefixed") references.

[Source](https://docs.rs/gix/latest/src/gix/repository/reference.rs.html#293-305)

#### pub fn [try\_find\_reference](#method.try_find_reference)<'a, Name, E>( &self, name: Name, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Reference](https://docs.rs/gix/latest/gix/struct.Reference.html "struct gix::Reference")<'\_>>, [Error](https://docs.rs/gix/latest/gix/reference/find/enum.Error.html "enum gix::reference::find::Error")\>

where Name: [TryInto](https://doc.rust-lang.org/nightly/core/convert/trait.TryInto.html "trait core::convert::TryInto")<&'a [PartialNameRef](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/struct.PartialNameRef.html "struct gix_ref::PartialNameRef"), Error = E>, [Error](https://docs.rs/gix-ref/0.52.1/x86_64-unknown-linux-gnu/gix_ref/store_impl/file/find/error/enum.Error.html "enum gix_ref::store_impl::file::find::error::Error"): [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<E>,

Try to find the reference named `name`, like `main`, `heads/branch`, `HEAD` or `origin/other`, and return it.

Otherwise return `None` if the reference wasn’t found. If the reference is expected to exist, use [`find_reference()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.find_reference "method gix::Repository::find_reference").

[Source](https://docs.rs/gix/latest/src/gix/repository/remote.rs.html#4-227)[§](#impl-Repository-25)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/remote.rs.html#9-15)

#### pub fn [remote\_at](#method.remote_at)<Url, E>(&self, url: Url) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Remote](https://docs.rs/gix/latest/gix/struct.Remote.html "struct gix::Remote")<'\_>, [Error](https://docs.rs/gix/latest/gix/remote/init/enum.Error.html "enum gix::remote::init::Error")\>

where Url: [TryInto](https://doc.rust-lang.org/nightly/core/convert/trait.TryInto.html "trait core::convert::TryInto")<[Url](https://docs.rs/gix/latest/gix/struct.Url.html "struct gix::Url"), Error = E>, [Error](https://docs.rs/gix-url/0.31.0/x86_64-unknown-linux-gnu/gix_url/parse/enum.Error.html "enum gix_url::parse::Error"): [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<E>,

Create a new remote available at the given `url`.

It’s configured to fetch included tags by default, similar to git. See [`with_fetch_tags(…)`](https://docs.rs/gix/latest/gix/struct.Remote.html#method.with_fetch_tags "method gix::Remote::with_fetch_tags") for a way to change it.

[Source](https://docs.rs/gix/latest/src/gix/repository/remote.rs.html#21-27)

#### pub fn [remote\_at\_without\_url\_rewrite](#method.remote_at_without_url_rewrite)<Url, E>( &self, url: Url, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Remote](https://docs.rs/gix/latest/gix/struct.Remote.html "struct gix::Remote")<'\_>, [Error](https://docs.rs/gix/latest/gix/remote/init/enum.Error.html "enum gix::remote::init::Error")\>

where Url: [TryInto](https://doc.rust-lang.org/nightly/core/convert/trait.TryInto.html "trait core::convert::TryInto")<[Url](https://docs.rs/gix/latest/gix/struct.Url.html "struct gix::Url"), Error = E>, [Error](https://docs.rs/gix-url/0.31.0/x86_64-unknown-linux-gnu/gix_url/parse/enum.Error.html "enum gix_url::parse::Error"): [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<E>,

Create a new remote available at the given `url` similarly to [`remote_at()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.remote_at "method gix::Repository::remote_at"), but don’t rewrite the url according to rewrite rules. This eliminates a failure mode in case the rewritten URL is faulty, allowing to selectively [apply rewrite rules](https://docs.rs/gix/latest/gix/struct.Remote.html#method.rewrite_urls "method gix::Remote::rewrite_urls") later and do so non-destructively.

[Source](https://docs.rs/gix/latest/src/gix/repository/remote.rs.html#33-40)

#### pub fn [find\_remote](#method.find_remote)<'a>( &self, name\_or\_url: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<&'a [BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Remote](https://docs.rs/gix/latest/gix/struct.Remote.html "struct gix::Remote")<'\_>, [Error](https://docs.rs/gix/latest/gix/remote/find/existing/enum.Error.html "enum gix::remote::find::existing::Error")\>

Find the configured remote with the given `name_or_url` or report an error, similar to [`try_find_remote(…)`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.try_find_remote "method gix::Repository::try_find_remote").

Note that we will obtain remotes only if we deem them [trustworthy](https://docs.rs/gix/latest/gix/open/struct.Options.html#method.filter_config_section "method gix::open::Options::filter_config_section").

[Source](https://docs.rs/gix/latest/src/gix/repository/remote.rs.html#45-51)

#### pub fn [find\_default\_remote](#method.find_default_remote)( &self, direction: [Direction](https://docs.rs/gix/latest/gix/remote/enum.Direction.html "enum gix::remote::Direction"), ) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Remote](https://docs.rs/gix/latest/gix/struct.Remote.html "struct gix::Remote")<'\_>, [Error](https://docs.rs/gix/latest/gix/remote/find/existing/enum.Error.html "enum gix::remote::find::existing::Error")\>>

Find the default remote as configured, or `None` if no such configuration could be found.

See [`remote_default_name()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.remote_default_name "method gix::Repository::remote_default_name") for more information on the `direction` parameter.

[Source](https://docs.rs/gix/latest/src/gix/repository/remote.rs.html#63-65)

#### pub fn [try\_find\_remote](#method.try_find_remote)<'a>( &self, name\_or\_url: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<&'a [BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>, ) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Remote](https://docs.rs/gix/latest/gix/struct.Remote.html "struct gix::Remote")<'\_>, [Error](https://docs.rs/gix/latest/gix/remote/find/enum.Error.html "enum gix::remote::find::Error")\>>

Find the configured remote with the given `name_or_url` or return `None` if it doesn’t exist, for the purpose of fetching or pushing data.

There are various error kinds related to partial information or incorrectly formatted URLs or ref-specs. Also note that the created `Remote` may have neither fetch nor push ref-specs set at all.

Note that ref-specs are de-duplicated right away which may change their order. This doesn’t affect matching in any way as negations/excludes are applied after includes.

We will only include information if we deem it [trustworthy](https://docs.rs/gix/latest/gix/open/struct.Options.html#method.filter_config_section "method gix::open::Options::filter_config_section").

[Source](https://docs.rs/gix/latest/src/gix/repository/remote.rs.html#80-94)

#### pub fn [find\_fetch\_remote](#method.find_fetch_remote)( &self, name\_or\_url: [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<&[BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Remote](https://docs.rs/gix/latest/gix/struct.Remote.html "struct gix::Remote")<'\_>, [Error](https://docs.rs/gix/latest/gix/remote/find/for_fetch/enum.Error.html "enum gix::remote::find::for_fetch::Error")\>

This method emulate what `git fetch <remote>` does in order to obtain a remote to fetch from.

As such, with `name_or_url` being `Some`, it will:

*   use `name_or_url` verbatim if it is a URL, creating a new remote in memory as needed.
*   find the named remote if `name_or_url` is a remote name

If `name_or_url` is `None`:

*   use the current `HEAD` branch to find a configured remote
*   fall back to either a generally configured remote or the only configured remote.

Fail if no remote could be found despite all of the above.

[Source](https://docs.rs/gix/latest/src/gix/repository/remote.rs.html#99-104)

#### pub fn [try\_find\_remote\_without\_url\_rewrite](#method.try_find_remote_without_url_rewrite)<'a>( &self, name\_or\_url: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<&'a [BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>, ) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Remote](https://docs.rs/gix/latest/gix/struct.Remote.html "struct gix::Remote")<'\_>, [Error](https://docs.rs/gix/latest/gix/remote/find/enum.Error.html "enum gix::remote::find::Error")\>>

Similar to [`try_find_remote()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.try_find_remote "method gix::Repository::try_find_remote"), but removes a failure mode if rewritten URLs turn out to be invalid as it skips rewriting them. Use this in conjunction with [`Remote::rewrite_urls()`](https://docs.rs/gix/latest/gix/struct.Remote.html#method.rewrite_urls "method gix::Remote::rewrite_urls") to non-destructively apply the rules and keep the failed urls unchanged.

[Source](https://docs.rs/gix/latest/src/gix/repository/revision.rs.html#6-150)[§](#impl-Repository-26)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Methods for resolving revisions by spec or working with the commit graph.

[Source](https://docs.rs/gix/latest/src/gix/repository/revision.rs.html#15-24)

#### pub fn [rev\_parse](#method.rev_parse)<'a>( &self, spec: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<&'a [BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Spec](https://docs.rs/gix/latest/gix/revision/struct.Spec.html "struct gix::revision::Spec")<'\_>, [Error](https://docs.rs/gix/latest/gix/revision/spec/parse/enum.Error.html "enum gix::revision::spec::parse::Error")\>

Available on **crate feature `revision`** only.

Parse a revision specification and turn it into the object(s) it describes, similar to `git rev-parse`.

##### [§](#deviation-1)Deviation

*   `@` actually stands for `HEAD`, whereas `git` resolves it to the object pointed to by `HEAD` without making the `HEAD` ref available for lookups.

[Source](https://docs.rs/gix/latest/src/gix/repository/revision.rs.html#29-37)

#### pub fn [rev\_parse\_single](#method.rev_parse_single)<'repo, 'a>( &'repo self, spec: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<&'a [BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'repo>, [Error](https://docs.rs/gix/latest/gix/revision/spec/parse/single/enum.Error.html "enum gix::revision::spec::parse::single::Error")\>

Available on **crate feature `revision`** only.

Parse a revision specification and return single object id as represented by this instance.

[Source](https://docs.rs/gix/latest/src/gix/repository/revision.rs.html#45-60)

#### pub fn [merge\_base](#method.merge_base)( &self, one: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, two: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'\_>, [Error](https://docs.rs/gix/latest/gix/repository/merge_base/enum.Error.html "enum gix::repository::merge_base::Error")\>

Available on **crate feature `revision`** only.

Obtain the best merge-base between commit `one` and `two`, or fail if there is none.

##### [§](#performance-5)Performance

For repeated calls, prefer [`merge_base_with_cache()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.merge_base_with_graph "method gix::Repository::merge_base_with_graph"). Also be sure to [set an object cache](https://docs.rs/gix/latest/gix/struct.Repository.html#method.object_cache_size_if_unset "method gix::Repository::object_cache_size_if_unset") to accelerate repeated commit lookups.

[Source](https://docs.rs/gix/latest/src/gix/repository/revision.rs.html#68-83)

#### pub fn [merge\_base\_with\_graph](#method.merge_base_with_graph)( &self, one: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, two: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, graph: &mut [Graph](https://docs.rs/gix-revwalk/0.20.1/x86_64-unknown-linux-gnu/gix_revwalk/struct.Graph.html "struct gix_revwalk::Graph")<'\_, '\_, [Commit](https://docs.rs/gix-revwalk/0.20.1/x86_64-unknown-linux-gnu/gix_revwalk/graph/struct.Commit.html "struct gix_revwalk::graph::Commit")<[Flags](https://docs.rs/gix-revision/0.34.1/x86_64-unknown-linux-gnu/gix_revision/merge_base/struct.Flags.html "struct gix_revision::merge_base::Flags")\>>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'\_>, [Error](https://docs.rs/gix/latest/gix/repository/merge_base_with_graph/enum.Error.html "enum gix::repository::merge_base_with_graph::Error")\>

Available on **crate feature `revision`** only.

Obtain the best merge-base between commit `one` and `two`, or fail if there is none, providing a commit-graph `graph` to potentially greatly accelerate the operation by reusing graphs from previous runs.

##### [§](#performance-6)Performance

Be sure to [set an object cache](https://docs.rs/gix/latest/gix/struct.Repository.html#method.object_cache_size_if_unset "method gix::Repository::object_cache_size_if_unset") to accelerate repeated commit lookups.

[Source](https://docs.rs/gix/latest/src/gix/repository/revision.rs.html#92-105)

#### pub fn [merge\_bases\_many\_with\_graph](#method.merge_bases_many_with_graph)( &self, one: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, others: &\[[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\], graph: &mut [Graph](https://docs.rs/gix-revwalk/0.20.1/x86_64-unknown-linux-gnu/gix_revwalk/struct.Graph.html "struct gix_revwalk::Graph")<'\_, '\_, [Commit](https://docs.rs/gix-revwalk/0.20.1/x86_64-unknown-linux-gnu/gix_revwalk/graph/struct.Commit.html "struct gix_revwalk::graph::Commit")<[Flags](https://docs.rs/gix-revision/0.34.1/x86_64-unknown-linux-gnu/gix_revision/merge_base/struct.Flags.html "struct gix_revision::merge_base::Flags")\>>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'\_>>, [Error](https://docs.rs/gix-revision/0.34.1/x86_64-unknown-linux-gnu/gix_revision/merge_base/enum.Error.html "enum gix_revision::merge_base::Error")\>

Available on **crate feature `revision`** only.

Obtain all merge-bases between commit `one` and `others`, or an empty list if there is none, providing a commit-graph `graph` to potentially greatly accelerate the operation.

##### [§](#performance-7)Performance

Be sure to [set an object cache](https://docs.rs/gix/latest/gix/struct.Repository.html#method.object_cache_size_if_unset "method gix::Repository::object_cache_size_if_unset") to accelerate repeated commit lookups.

[Source](https://docs.rs/gix/latest/src/gix/repository/revision.rs.html#111-125)

#### pub fn [merge\_base\_octopus\_with\_graph](#method.merge_base_octopus_with_graph)( &self, commits: impl [IntoIterator](https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")<Item = impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>>, graph: &mut [Graph](https://docs.rs/gix-revwalk/0.20.1/x86_64-unknown-linux-gnu/gix_revwalk/struct.Graph.html "struct gix_revwalk::Graph")<'\_, '\_, [Commit](https://docs.rs/gix-revwalk/0.20.1/x86_64-unknown-linux-gnu/gix_revwalk/graph/struct.Commit.html "struct gix_revwalk::graph::Commit")<[Flags](https://docs.rs/gix-revision/0.34.1/x86_64-unknown-linux-gnu/gix_revision/merge_base/struct.Flags.html "struct gix_revision::merge_base::Flags")\>>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'\_>, [Error](https://docs.rs/gix/latest/gix/repository/merge_base_octopus_with_graph/enum.Error.html "enum gix::repository::merge_base_octopus_with_graph::Error")\>

Available on **crate feature `revision`** only.

Return the best merge-base among all `commits`, or fail if `commits` yields no commit or no merge-base was found.

Use `graph` to speed up repeated calls.

[Source](https://docs.rs/gix/latest/src/gix/repository/revision.rs.html#131-138)

#### pub fn [merge\_base\_octopus](#method.merge_base_octopus)( &self, commits: impl [IntoIterator](https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")<Item = impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Id](https://docs.rs/gix/latest/gix/struct.Id.html "struct gix::Id")<'\_>, [Error](https://docs.rs/gix/latest/gix/repository/merge_base_octopus/enum.Error.html "enum gix::repository::merge_base_octopus::Error")\>

Available on **crate feature `revision`** only.

Return the best merge-base among all `commits`, or fail if `commits` yields no commit or no merge-base was found.

For repeated calls, prefer [`Self::merge_base_octopus_with_graph()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.merge_base_octopus_with_graph "method gix::Repository::merge_base_octopus_with_graph") for cache-reuse.

[Source](https://docs.rs/gix/latest/src/gix/repository/revision.rs.html#144-149)

#### pub fn [rev\_walk](#method.rev_walk)( &self, tips: impl [IntoIterator](https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")<Item = impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>>, ) -> [Platform](https://docs.rs/gix/latest/gix/revision/walk/struct.Platform.html "struct gix::revision::walk::Platform")<'\_>

Create the baseline for a revision walk by initializing it with the `tips` to start iterating on.

It can be configured further before starting the actual walk.

[Source](https://docs.rs/gix/latest/src/gix/repository/shallow.rs.html#5-38)[§](#impl-Repository-27)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/shallow.rs.html#7-9)

#### pub fn [is\_shallow](#method.is_shallow)(&self) -> [bool](https://doc.rust-lang.org/nightly/std/primitive.bool.html)

Return `true` if the repository is a shallow clone, i.e. contains history only up to a certain depth.

[Source](https://docs.rs/gix/latest/src/gix/repository/shallow.rs.html#19-24)

#### pub fn [shallow\_commits](#method.shallow_commits)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Commits](https://docs.rs/gix/latest/gix/shallow/type.Commits.html "type gix::shallow::Commits")\>, [Error](https://docs.rs/gix/latest/gix/shallow/read/enum.Error.html "enum gix::shallow::read::Error")\>

Return a shared list of shallow commits which is updated automatically if the in-memory snapshot has become stale as the underlying file on disk has changed.

The list of shallow commits represents the shallow boundary, beyond which we are lacking all (parent) commits. Note that the list is never empty, as `Ok(None)` is returned in that case indicating the repository isn’t a shallow clone.

The shared list is shared across all clones of this repository.

[Source](https://docs.rs/gix/latest/src/gix/repository/shallow.rs.html#30-37)

#### pub fn [shallow\_file](#method.shallow_file)(&self) -> [PathBuf](https://doc.rust-lang.org/nightly/std/path/struct.PathBuf.html "struct std::path::PathBuf")

Return the path to the `shallow` file which contains hashes, one per line, that describe commits that don’t have their parents within this repository.

Note that it may not exist if the repository isn’t actually shallow.

[Source](https://docs.rs/gix/latest/src/gix/repository/state.rs.html#3-44)[§](#impl-Repository-28)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/state.rs.html#8-43)

#### pub fn [state](#method.state)(&self) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[InProgress](https://docs.rs/gix/latest/gix/state/enum.InProgress.html "enum gix::state::InProgress")\>

Returns the status of an in progress operation on a repository or [`None`](https://doc.rust-lang.org/nightly/core/option/enum.Option.html#variant.None "variant core::option::Option::None") if no operation is currently in progress.

Note to be confused with the repositories ‘status’.

[Source](https://docs.rs/gix/latest/src/gix/repository/submodule.rs.html#5-96)[§](#impl-Repository-29)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/submodule.rs.html#11-27)

#### pub fn [open\_modules\_file](#method.open_modules_file)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[File](https://docs.rs/gix/latest/gix/submodule/struct.File.html "struct gix::submodule::File")\>, [Error](https://docs.rs/gix/latest/gix/submodule/open_modules_file/enum.Error.html "enum gix::submodule::open_modules_file::Error")\>

Available on **crate feature `attributes`** only.

Open the `.gitmodules` file as present in the worktree, or return `None` if no such file is available. Note that git configuration is also contributing to the result based on the current snapshot.

Note that his method will not look in other places, like the index or the `HEAD` tree.

[Source](https://docs.rs/gix/latest/src/gix/repository/submodule.rs.html#40-73)

#### pub fn [modules](#method.modules)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[ModulesSnapshot](https://docs.rs/gix/latest/gix/submodule/type.ModulesSnapshot.html "type gix::submodule::ModulesSnapshot")\>, [Error](https://docs.rs/gix/latest/gix/submodule/modules/enum.Error.html "enum gix::submodule::modules::Error")\>

Available on **crate feature `attributes`** only.

Return a shared [`.gitmodules` file](https://docs.rs/gix/latest/gix/submodule/struct.File.html "struct gix::submodule::File") which is updated automatically if the in-memory snapshot has become stale as the underlying file on disk has changed. The snapshot based on the file on disk is shared across all clones of this repository.

If a file on disk isn’t present, we will try to load it from the index, and finally from the current tree. In the latter two cases, the result will not be cached in this repository instance as we can’t detect freshness anymore, so time this method is called a new [modules file](https://docs.rs/gix/latest/gix/submodule/type.ModulesSnapshot.html "type gix::submodule::ModulesSnapshot") will be created.

Note that git configuration is also contributing to the result based on the current snapshot.

[Source](https://docs.rs/gix/latest/src/gix/repository/submodule.rs.html#77-95)

#### pub fn [submodules](#method.submodules)( &self, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<impl [Iterator](https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html "trait core::iter::traits::iterator::Iterator")<Item = [Submodule](https://docs.rs/gix/latest/gix/struct.Submodule.html "struct gix::Submodule")<'\_>>>, [Error](https://docs.rs/gix/latest/gix/submodule/modules/enum.Error.html "enum gix::submodule::modules::Error")\>

Available on **crate feature `attributes`** only.

Return the list of available submodules, or `None` if there is no submodule configuration.

[Source](https://docs.rs/gix/latest/src/gix/repository/worktree.rs.html#4-145)[§](#impl-Repository-30)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Interact with individual worktrees and their information.

[Source](https://docs.rs/gix/latest/src/gix/repository/worktree.rs.html#13-32)

#### pub fn [worktrees](#method.worktrees)(&self) -> [Result](https://doc.rust-lang.org/nightly/std/io/error/type.Result.html "type std::io::error::Result")<[Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[Proxy](https://docs.rs/gix/latest/gix/worktree/struct.Proxy.html "struct gix::worktree::Proxy")<'\_>>>

Return a list of all **linked** worktrees sorted by private git dir path as a lightweight proxy.

This means the number is `0` even if there is the main worktree, as it is not counted as linked worktree. This also means it will be `1` if there is one linked worktree next to the main worktree. It’s worth noting that a _bare_ repository may have one or more linked worktrees, but has no _main_ worktree, which is the reason why the _possibly_ available main worktree isn’t listed here.

Note that these need additional processing to become usable, but provide a first glimpse a typical worktree information.

[Source](https://docs.rs/gix/latest/src/gix/repository/worktree.rs.html#38-40)

#### pub fn [main\_repo](#method.main_repo)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository"), [Error](https://docs.rs/gix/latest/gix/open/enum.Error.html "enum gix::open::Error")\>

Return the repository owning the main worktree, typically from a linked worktree.

Note that it might be the one that is currently open if this repository doesn’t point to a linked worktree. Also note that the main repo might be bare.

[Source](https://docs.rs/gix/latest/src/gix/repository/worktree.rs.html#47-49)

#### pub fn [worktree](#method.worktree)(&self) -> [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Worktree](https://docs.rs/gix/latest/gix/struct.Worktree.html "struct gix::Worktree")<'\_>>

Return the currently set worktree if there is one, acting as platform providing a validated worktree base path.

Note that there would be `None` if this repository is `bare` and the parent [`Repository`](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository") was instantiated without registered worktree in the current working dir, even if no `.git` file or directory exists. It’s merely based on configuration, see [Worktree::dot\_git\_exists()](https://docs.rs/gix/latest/gix/struct.Worktree.html#method.dot_git_exists "method gix::Worktree::dot_git_exists") for a way to perform more validation.

[Source](https://docs.rs/gix/latest/src/gix/repository/worktree.rs.html#55-57)

#### pub fn [is\_bare](#method.is_bare)(&self) -> [bool](https://doc.rust-lang.org/nightly/std/primitive.bool.html)

Return true if this repository is bare, and has no main work tree.

This is not to be confused with the [`worktree()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.worktree "method gix::Repository::worktree") worktree, which may exists if this instance was opened in a worktree that was created separately.

[Source](https://docs.rs/gix/latest/src/gix/repository/worktree.rs.html#65-99)

#### pub fn [worktree\_stream](#method.worktree_stream)( &self, id: impl [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId")\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<([Stream](https://docs.rs/gix-worktree-stream/0.21.1/x86_64-unknown-linux-gnu/gix_worktree_stream/struct.Stream.html "struct gix_worktree_stream::Stream"), [File](https://docs.rs/gix/latest/gix/index/struct.File.html "struct gix::index::File")), [Error](https://docs.rs/gix/latest/gix/repository/worktree_stream/enum.Error.html "enum gix::repository::worktree_stream::Error")\>

Available on **crate feature `worktree-stream`** only.

If `id` points to a tree, produce a stream that yields one worktree entry after the other. The index of the tree at `id` is returned as well as it is an intermediate byproduct that might be useful to callers.

The entries will look exactly like they would if one would check them out, with filters applied. The `export-ignore` attribute is used to skip blobs or directories to which it applies.

[Source](https://docs.rs/gix/latest/src/gix/repository/worktree.rs.html#114-144)

#### pub fn [worktree\_archive](#method.worktree_archive)( &self, stream: [Stream](https://docs.rs/gix-worktree-stream/0.21.1/x86_64-unknown-linux-gnu/gix_worktree_stream/struct.Stream.html "struct gix_worktree_stream::Stream"), out: impl [Write](https://doc.rust-lang.org/nightly/std/io/trait.Write.html "trait std::io::Write") + [Seek](https://doc.rust-lang.org/nightly/std/io/trait.Seek.html "trait std::io::Seek"), blobs: impl [Count](https://docs.rs/gix/latest/gix/trait.Count.html "trait gix::Count"), should\_interrupt: &[AtomicBool](https://doc.rust-lang.org/nightly/core/sync/atomic/struct.AtomicBool.html "struct core::sync::atomic::AtomicBool"), options: [Options](https://docs.rs/gix-archive/0.21.1/x86_64-unknown-linux-gnu/gix_archive/struct.Options.html "struct gix_archive::Options"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[()](https://doc.rust-lang.org/nightly/std/primitive.unit.html), [Error](https://docs.rs/gix/latest/gix/repository/worktree_archive/type.Error.html "type gix::repository::worktree_archive::Error")\>

Available on **crate feature `worktree-archive`** only.

Produce an archive from the `stream` and write it to `out` according to `options`. Use `blob` to provide progress for each entry written to `out`, and note that it should already be initialized to the amount of expected entries, with `should_interrupt` being queried between each entry to abort if needed, and on each write to `out`.

###### [§](#performance-8)Performance

Be sure that `out` is able to handle a lot of write calls. Otherwise wrap it in a [`BufWriter`](https://doc.rust-lang.org/nightly/std/io/buffered/bufwriter/struct.BufWriter.html "struct std::io::buffered::bufwriter::BufWriter").

###### [§](#additional-progress-and-fine-grained-interrupt-handling)Additional progress and fine-grained interrupt handling

For additional progress reporting, wrap `out` into a writer that counts throughput on each write. This can also be used to react to interrupts on each write, instead of only for each entry.

[Source](https://docs.rs/gix/latest/src/gix/status/mod.rs.html#155-200)[§](#impl-Repository-31)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/status/mod.rs.html#167-199)

#### pub fn [is\_dirty](#method.is_dirty)(&self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[bool](https://doc.rust-lang.org/nightly/std/primitive.bool.html), [Error](https://docs.rs/gix/latest/gix/status/is_dirty/enum.Error.html "enum gix::status::is_dirty::Error")\>

Available on **crate feature `status`** only.

Returns `true` if the repository is dirty. This means it’s changed in one of the following ways:

*   the index was changed in comparison to its working tree
*   the working tree was changed in comparison to the index
*   submodules are taken in consideration, along with their `ignore` and `isActive` configuration

Note that _untracked files_ do _not_ affect this flag.

[Source](https://docs.rs/gix/latest/src/gix/status/index_worktree.rs.html#64-191)[§](#impl-Repository-32)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/status/index_worktree.rs.html#89-165)

#### pub fn [index\_worktree\_status](#method.index_worktree_status)<'index, T, U, E>( &self, index: &'index [State](https://docs.rs/gix/latest/gix/index/struct.State.html "struct gix::index::State"), patterns: impl [IntoIterator](https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")<Item = impl [AsRef](https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html "trait core::convert::AsRef")<[BStr](https://docs.rs/gix/latest/gix/diff/object/bstr/struct.BStr.html "struct gix::diff::object::bstr::BStr")\>>, delegate: &mut impl [VisitEntry](https://docs.rs/gix-status/0.19.1/x86_64-unknown-linux-gnu/gix_status/index_as_worktree_with_renames/types/trait.VisitEntry.html "trait gix_status::index_as_worktree_with_renames::types::VisitEntry")<'index, ContentChange = T, SubmoduleStatus = U>, compare: impl [CompareBlobs](https://docs.rs/gix-status/0.19.1/x86_64-unknown-linux-gnu/gix_status/index_as_worktree/traits/trait.CompareBlobs.html "trait gix_status::index_as_worktree::traits::CompareBlobs")<Output = T> + [Send](https://doc.rust-lang.org/nightly/core/marker/trait.Send.html "trait core::marker::Send") + [Clone](https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html "trait core::clone::Clone"), submodule: impl [SubmoduleStatus](https://docs.rs/gix-status/0.19.1/x86_64-unknown-linux-gnu/gix_status/index_as_worktree/traits/trait.SubmoduleStatus.html "trait gix_status::index_as_worktree::traits::SubmoduleStatus")<Output = U, Error = E> + [Send](https://doc.rust-lang.org/nightly/core/marker/trait.Send.html "trait core::marker::Send") + [Clone](https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html "trait core::clone::Clone"), progress: &mut dyn [Progress](https://docs.rs/gix/latest/gix/trait.Progress.html "trait gix::Progress"), should\_interrupt: &[AtomicBool](https://doc.rust-lang.org/nightly/core/sync/atomic/struct.AtomicBool.html "struct core::sync::atomic::AtomicBool"), options: [Options](https://docs.rs/gix/latest/gix/status/index_worktree/struct.Options.html "struct gix::status::index_worktree::Options"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Outcome](https://docs.rs/gix-status/0.19.1/x86_64-unknown-linux-gnu/gix_status/index_as_worktree_with_renames/types/struct.Outcome.html "struct gix_status::index_as_worktree_with_renames::types::Outcome"), [Error](https://docs.rs/gix/latest/gix/status/index_worktree/enum.Error.html "enum gix::status::index_worktree::Error")\>

where T: [Send](https://doc.rust-lang.org/nightly/core/marker/trait.Send.html "trait core::marker::Send") + [Clone](https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html "trait core::clone::Clone"), U: [Send](https://doc.rust-lang.org/nightly/core/marker/trait.Send.html "trait core::marker::Send") + [Clone](https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html "trait core::clone::Clone"), E: [Error](https://doc.rust-lang.org/nightly/core/error/trait.Error.html "trait core::error::Error") + [Send](https://doc.rust-lang.org/nightly/core/marker/trait.Send.html "trait core::marker::Send") + [Sync](https://doc.rust-lang.org/nightly/core/marker/trait.Sync.html "trait core::marker::Sync") + 'static,

Available on **crate feature `status`** only.

Obtain the status between the index and the worktree, involving modification checks for all tracked files along with information about untracked (and posisbly ignored) files (if configured).

*   `index`
    *   The index to use for modification checks, and to know which files are tacked when applying the dirwalk.
*   `patterns`
    *   Optional patterns to use to limit the paths to look at. If empty, all paths are considered.
*   `delegate`
    *   The sink for receiving all status data.
*   `compare`
    *   The implementations for fine-grained control over what happens if a hash must be recalculated.
*   `submodule`
    *   Control what kind of information to retrieve when a submodule is encountered while traversing the index.
*   `progress`
    *   A progress indication for index modification checks.
*   `should_interrupt`
    *   A flag to stop the whole operation.
*   `options`
    *   Additional configuration for all parts of the operation.

###### [§](#note-5)Note

This is a lower-level method, prefer the [`status`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.status "method gix::Repository::status") method for greater ease of use.

[Source](https://docs.rs/gix/latest/src/gix/status/tree_index.rs.html#43-139)[§](#impl-Repository-33)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/status/tree_index.rs.html#55-138)

#### pub fn [tree\_index\_status](#method.tree_index_status)<'repo, E>( &'repo self, tree\_id: &[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid"), worktree\_index: &[State](https://docs.rs/gix/latest/gix/index/struct.State.html "struct gix::index::State"), pathspec: [Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<&mut [Pathspec](https://docs.rs/gix/latest/gix/struct.Pathspec.html "struct gix::Pathspec")<'repo>>, renames: [TrackRenames](https://docs.rs/gix/latest/gix/status/tree_index/enum.TrackRenames.html "enum gix::status::tree_index::TrackRenames"), cb: impl [FnMut](https://doc.rust-lang.org/nightly/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")([ChangeRef](https://docs.rs/gix/latest/gix/diff/index/enum.ChangeRef.html "enum gix::diff::index::ChangeRef")<'\_, '\_>, &[State](https://docs.rs/gix/latest/gix/index/struct.State.html "struct gix::index::State"), &[State](https://docs.rs/gix/latest/gix/index/struct.State.html "struct gix::index::State")) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Action](https://docs.rs/gix/latest/gix/diff/index/enum.Action.html "enum gix::diff::index::Action"), E>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Outcome](https://docs.rs/gix/latest/gix/status/tree_index/struct.Outcome.html "struct gix::status::tree_index::Outcome"), [Error](https://docs.rs/gix/latest/gix/status/tree_index/enum.Error.html "enum gix::status::tree_index::Error")\>

where E: [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<[Box](https://doc.rust-lang.org/nightly/alloc/boxed/struct.Box.html "struct alloc::boxed::Box")<dyn [Error](https://doc.rust-lang.org/nightly/core/error/trait.Error.html "trait core::error::Error") + [Send](https://doc.rust-lang.org/nightly/core/marker/trait.Send.html "trait core::marker::Send") + [Sync](https://doc.rust-lang.org/nightly/core/marker/trait.Sync.html "trait core::marker::Sync")\>>,

Available on **crate feature `status`** only.

Produce the `git status` portion that shows the difference between `tree_id` (usually `HEAD^{tree}`) and the `worktree_index` (typically the current `.git/index`), and pass all changes to `cb(change, tree_index, worktree_index)` with full access to both indices that contributed to the change.

_(It’s notable that internally, the `tree_id` is converted into an index before diffing these)_. Set `pathspec` to `Some(_)` to further reduce the set of files to check.

###### [§](#notes-2)Notes

*   This is a low-level method - prefer the [`Repository::status()`](https://docs.rs/gix/latest/gix/struct.Repository.html#method.status "method gix::Repository::status") platform instead for access to various iterators over the same information.

[Source](https://docs.rs/gix/latest/src/gix/status/mod.rs.html#76-131)[§](#impl-Repository-34)

### impl [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Status

[Source](https://docs.rs/gix/latest/src/gix/status/mod.rs.html#98-130)

#### pub fn [status](#method.status)<P>(&self, progress: P) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Platform](https://docs.rs/gix/latest/gix/status/struct.Platform.html "struct gix::status::Platform")<'\_, P>, [Error](https://docs.rs/gix/latest/gix/status/enum.Error.html "enum gix::status::Error")\>

where P: [Progress](https://docs.rs/gix/latest/gix/trait.Progress.html "trait gix::Progress") + 'static,

Available on **crate feature `status`** only.

Obtain a platform for configuring iterators for traversing git repository status information.

By default, this is set to the fastest and most immediate way of obtaining a status, which is most similar to

`git status --ignored=no`

which implies that submodule information is provided by default.

Note that `status.showUntrackedFiles` is respected, which leads to untracked files being collapsed by default. If that needs to be controlled, [configure the directory walk explicitly](https://docs.rs/gix/latest/gix/status/struct.Platform.html#method.dirwalk_options "method gix::status::Platform::dirwalk_options") or more [implicitly](https://docs.rs/gix/latest/gix/status/struct.Platform.html#method.untracked_files "method gix::status::Platform::untracked_files").

Pass `progress` to receive progress information on file modifications on this repository. Use [`progress::Discard`](https://docs.rs/gix/latest/gix/progress/struct.Discard.html "struct gix::progress::Discard") to discard all progress information.

###### [§](#deviation-2)Deviation

Whereas Git runs the index-modified check before the directory walk to set entries as up-to-date to (potentially) safe some disk-access, we run both in parallel which ultimately is much faster.

Trait Implementations[§](#trait-implementations)
------------------------------------------------

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#6-28)[§](#impl-Clone-for-Repository)

### impl [Clone](https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html "trait core::clone::Clone") for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#7-27)[§](#method.clone)

#### fn [clone](https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#tymethod.clone)(&self) -> Self

Returns a duplicate of the value. [Read more](https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#tymethod.clone)

1.0.0 · [Source](https://doc.rust-lang.org/nightly/src/core/clone.rs.html#213-215)[§](#method.clone_from)

#### const fn [clone\_from](https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#method.clone_from)(&mut self, source: &Self)

Performs copy-assignment from `source`. [Read more](https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#method.clone_from)

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#30-38)[§](#impl-Debug-for-Repository)

### impl [Debug](https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html "trait core::fmt::Debug") for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#31-37)[§](#method.fmt)

#### fn [fmt](https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html#tymethod.fmt)(&self, f: &mut [Formatter](https://doc.rust-lang.org/nightly/core/fmt/struct.Formatter.html "struct core::fmt::Formatter")<'\_>) -> [Result](https://doc.rust-lang.org/nightly/core/fmt/type.Result.html "type core::fmt::Result")

Formats the value using the given formatter. [Read more](https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html#tymethod.fmt)

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#161-168)[§](#impl-Exists-for-Repository)

### impl [Exists](https://docs.rs/gix/latest/gix/diff/object/trait.Exists.html "trait gix::diff::object::Exists") for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#162-167)[§](#method.exists)

#### fn [exists](https://docs.rs/gix/latest/gix/diff/object/trait.Exists.html#tymethod.exists)(&self, id: &[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid")) -> [bool](https://doc.rust-lang.org/nightly/std/primitive.bool.html)

Returns `true` if the object exists in the database.

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#144-159)[§](#impl-Find-for-Repository)

### impl [Find](https://docs.rs/gix/latest/gix/prelude/trait.Find.html "trait gix::prelude::Find") for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#145-158)[§](#method.try_find)

#### fn [try\_find](https://docs.rs/gix/latest/gix/prelude/trait.Find.html#tymethod.try_find)<'a>( &self, id: &[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid"), buffer: &'a mut [Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html)\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Data](https://docs.rs/gix/latest/gix/diff/object/struct.Data.html "struct gix::diff::object::Data")<'a>>, [Error](https://docs.rs/gix/latest/gix/diff/object/find/type.Error.html "type gix::diff::object::find::Error")\>

Find an object matching `id` in the database while placing its raw, possibly encoded data into `buffer`. [Read more](https://docs.rs/gix/latest/gix/prelude/trait.Find.html#tymethod.try_find)

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#132-142)[§](#impl-FindHeader-for-Repository)

### impl [Header](https://docs.rs/gix/latest/gix/diff/object/trait.FindHeader.html "trait gix::diff::object::FindHeader") for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#133-141)[§](#method.try_header)

#### fn [try\_header](https://docs.rs/gix/latest/gix/diff/object/trait.FindHeader.html#tymethod.try_header)(&self, id: &[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid")) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Option](https://doc.rust-lang.org/nightly/core/option/enum.Option.html "enum core::option::Option")<[Header](https://docs.rs/gix/latest/gix/diff/object/struct.Header.html "struct gix::diff::object::Header")\>, [Error](https://docs.rs/gix/latest/gix/diff/object/find/type.Error.html "type gix::diff::object::find::Error")\>

Find the header of the object matching `id` in the database. [Read more](https://docs.rs/gix/latest/gix/diff/object/trait.FindHeader.html#tymethod.try_header)

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#48-64)[§](#impl-From%3C%26ThreadSafeRepository%3E-for-Repository)

### impl [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<&[ThreadSafeRepository](https://docs.rs/gix/latest/gix/struct.ThreadSafeRepository.html "struct gix::ThreadSafeRepository")\> for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#49-63)[§](#method.from-2)

#### fn [from](https://doc.rust-lang.org/nightly/core/convert/trait.From.html#tymethod.from)(repo: &[ThreadSafeRepository](https://docs.rs/gix/latest/gix/struct.ThreadSafeRepository.html "struct gix::ThreadSafeRepository")) -> Self

Converts to this type from the input type.

[Source](https://docs.rs/gix/latest/src/gix/clone/checkout.rs.html#179-183)[§](#impl-From%3CPrepareCheckout%3E-for-Repository)

### impl [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<[PrepareCheckout](https://docs.rs/gix/latest/gix/clone/struct.PrepareCheckout.html "struct gix::clone::PrepareCheckout")\> for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Available on **crate feature `worktree-mutation`** only.

[Source](https://docs.rs/gix/latest/src/gix/clone/checkout.rs.html#180-182)[§](#method.from-1)

#### fn [from](https://doc.rust-lang.org/nightly/core/convert/trait.From.html#tymethod.from)(prep: [PrepareCheckout](https://docs.rs/gix/latest/gix/clone/struct.PrepareCheckout.html "struct gix::clone::PrepareCheckout")) -> Self

Converts to this type from the input type.

[Source](https://docs.rs/gix/latest/src/gix/clone/access.rs.html#76-80)[§](#impl-From%3CPrepareFetch%3E-for-Repository)

### impl [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<[PrepareFetch](https://docs.rs/gix/latest/gix/clone/struct.PrepareFetch.html "struct gix::clone::PrepareFetch")\> for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/clone/access.rs.html#77-79)[§](#method.from)

#### fn [from](https://doc.rust-lang.org/nightly/core/convert/trait.From.html#tymethod.from)(prep: [PrepareFetch](https://docs.rs/gix/latest/gix/clone/struct.PrepareFetch.html "struct gix::clone::PrepareFetch")) -> Self

Converts to this type from the input type.

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#84-100)[§](#impl-From%3CRepository%3E-for-ThreadSafeRepository)

### impl [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<[Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")\> for [ThreadSafeRepository](https://docs.rs/gix/latest/gix/struct.ThreadSafeRepository.html "struct gix::ThreadSafeRepository")

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#85-99)[§](#method.from-4)

#### fn [from](https://doc.rust-lang.org/nightly/core/convert/trait.From.html#tymethod.from)(r: [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")) -> Self

Converts to this type from the input type.

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#66-82)[§](#impl-From%3CThreadSafeRepository%3E-for-Repository)

### impl [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<[ThreadSafeRepository](https://docs.rs/gix/latest/gix/struct.ThreadSafeRepository.html "struct gix::ThreadSafeRepository")\> for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#67-81)[§](#method.from-3)

#### fn [from](https://doc.rust-lang.org/nightly/core/convert/trait.From.html#tymethod.from)(repo: [ThreadSafeRepository](https://docs.rs/gix/latest/gix/struct.ThreadSafeRepository.html "struct gix::ThreadSafeRepository")) -> Self

Converts to this type from the input type.

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#40-46)[§](#impl-PartialEq-for-Repository)

### impl [PartialEq](https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html "trait core::cmp::PartialEq") for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#41-45)[§](#method.eq)

#### fn [eq](https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html#tymethod.eq)(&self, other: &[Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")) -> [bool](https://doc.rust-lang.org/nightly/std/primitive.bool.html)

Tests for `self` and `other` values to be equal, and is used by `==`.

1.0.0 · [Source](https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#265)[§](#method.ne)

#### const fn [ne](https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html#method.ne)(&self, other: [&Rhs](https://doc.rust-lang.org/nightly/std/primitive.reference.html)) -> [bool](https://doc.rust-lang.org/nightly/std/primitive.bool.html)

Tests for `!=`. The default implementation is almost always sufficient, and should not be overridden without very good reason.

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#102-130)[§](#impl-Write-for-Repository)

### impl [Write](https://docs.rs/gix/latest/gix/prelude/trait.Write.html "trait gix::prelude::Write") for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#103-107)[§](#method.write)

#### fn [write](https://docs.rs/gix/latest/gix/prelude/trait.Write.html#method.write)(&self, object: &dyn [WriteTo](https://docs.rs/gix/latest/gix/diff/object/trait.WriteTo.html "trait gix::diff::object::WriteTo")) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId"), [Error](https://docs.rs/gix/latest/gix/diff/object/write/type.Error.html "type gix::diff::object::write::Error")\>

Write objects using the intrinsic kind of [`hash`](https://docs.rs/gix/latest/gix/index/hash/enum.Kind.html "enum gix::index::hash::Kind") into the database, returning id to reference it in subsequent reads.

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#109-115)[§](#method.write_buf)

#### fn [write\_buf](https://docs.rs/gix/latest/gix/prelude/trait.Write.html#method.write_buf)(&self, object: [Kind](https://docs.rs/gix/latest/gix/object/enum.Kind.html "enum gix::object::Kind"), from: &\[[u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html)\]) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId"), [Error](https://docs.rs/gix/latest/gix/diff/object/write/type.Error.html "type gix::diff::object::write::Error")\>

As [`write`](https://docs.rs/gix/latest/gix/prelude/trait.Write.html#method.write "method gix::prelude::Write::write"), but takes an [`object` kind](https://docs.rs/gix/latest/gix/object/enum.Kind.html "enum gix::object::Kind") along with its encoded bytes.

[Source](https://docs.rs/gix/latest/src/gix/repository/impls.rs.html#117-129)[§](#method.write_stream)

#### fn [write\_stream](https://docs.rs/gix/latest/gix/prelude/trait.Write.html#tymethod.write_stream)( &self, kind: [Kind](https://docs.rs/gix/latest/gix/object/enum.Kind.html "enum gix::object::Kind"), size: [u64](https://doc.rust-lang.org/nightly/std/primitive.u64.html), from: &mut dyn [Read](https://doc.rust-lang.org/nightly/std/io/trait.Read.html "trait std::io::Read"), ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[ObjectId](https://docs.rs/gix/latest/gix/enum.ObjectId.html "enum gix::ObjectId"), [Error](https://docs.rs/gix/latest/gix/diff/object/write/type.Error.html "type gix::diff::object::write::Error")\>

As [`write`](https://docs.rs/gix/latest/gix/prelude/trait.Write.html#method.write "method gix::prelude::Write::write"), but takes an input stream. This is commonly used for writing blobs directly without reading them to memory first.

Auto Trait Implementations[§](#synthetic-implementations)
---------------------------------------------------------

[§](#impl-Freeze-for-Repository)

### impl ![Freeze](https://doc.rust-lang.org/nightly/core/marker/trait.Freeze.html "trait core::marker::Freeze") for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[§](#impl-RefUnwindSafe-for-Repository)

### impl ![RefUnwindSafe](https://doc.rust-lang.org/nightly/core/panic/unwind_safe/trait.RefUnwindSafe.html "trait core::panic::unwind_safe::RefUnwindSafe") for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[§](#impl-Send-for-Repository)

### impl [Send](https://doc.rust-lang.org/nightly/core/marker/trait.Send.html "trait core::marker::Send") for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[§](#impl-Sync-for-Repository)

### impl ![Sync](https://doc.rust-lang.org/nightly/core/marker/trait.Sync.html "trait core::marker::Sync") for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[§](#impl-Unpin-for-Repository)

### impl [Unpin](https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html "trait core::marker::Unpin") for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

[§](#impl-UnwindSafe-for-Repository)

### impl ![UnwindSafe](https://doc.rust-lang.org/nightly/core/panic/unwind_safe/trait.UnwindSafe.html "trait core::panic::unwind_safe::UnwindSafe") for [Repository](https://docs.rs/gix/latest/gix/struct.Repository.html "struct gix::Repository")

Blanket Implementations[§](#blanket-implementations)
----------------------------------------------------

[Source](https://doc.rust-lang.org/nightly/src/core/any.rs.html#138)[§](#impl-Any-for-T)

### impl<T> [Any](https://doc.rust-lang.org/nightly/core/any/trait.Any.html "trait core::any::Any") for T

where T: 'static + ?[Sized](https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](https://doc.rust-lang.org/nightly/src/core/any.rs.html#139)[§](#method.type_id)

#### fn [type\_id](https://doc.rust-lang.org/nightly/core/any/trait.Any.html#tymethod.type_id)(&self) -> [TypeId](https://doc.rust-lang.org/nightly/core/any/struct.TypeId.html "struct core::any::TypeId")

Gets the `TypeId` of `self`. [Read more](https://doc.rust-lang.org/nightly/core/any/trait.Any.html#tymethod.type_id)

[Source](https://doc.rust-lang.org/nightly/src/core/borrow.rs.html#209)[§](#impl-Borrow%3CT%3E-for-T)

### impl<T> [Borrow](https://doc.rust-lang.org/nightly/core/borrow/trait.Borrow.html "trait core::borrow::Borrow")<T> for T

where T: ?[Sized](https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](https://doc.rust-lang.org/nightly/src/core/borrow.rs.html#211)[§](#method.borrow)

#### fn [borrow](https://doc.rust-lang.org/nightly/core/borrow/trait.Borrow.html#tymethod.borrow)(&self) -> [&T](https://doc.rust-lang.org/nightly/std/primitive.reference.html)

Immutably borrows from an owned value. [Read more](https://doc.rust-lang.org/nightly/core/borrow/trait.Borrow.html#tymethod.borrow)

[Source](https://doc.rust-lang.org/nightly/src/core/borrow.rs.html#217)[§](#impl-BorrowMut%3CT%3E-for-T)

### impl<T> [BorrowMut](https://doc.rust-lang.org/nightly/core/borrow/trait.BorrowMut.html "trait core::borrow::BorrowMut")<T> for T

where T: ?[Sized](https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](https://doc.rust-lang.org/nightly/src/core/borrow.rs.html#218)[§](#method.borrow_mut)

#### fn [borrow\_mut](https://doc.rust-lang.org/nightly/core/borrow/trait.BorrowMut.html#tymethod.borrow_mut)(&mut self) -> [&mut T](https://doc.rust-lang.org/nightly/std/primitive.reference.html)

Mutably borrows from an owned value. [Read more](https://doc.rust-lang.org/nightly/core/borrow/trait.BorrowMut.html#tymethod.borrow_mut)

[Source](https://doc.rust-lang.org/nightly/src/core/clone.rs.html#483)[§](#impl-CloneToUninit-for-T)

### impl<T> [CloneToUninit](https://doc.rust-lang.org/nightly/core/clone/trait.CloneToUninit.html "trait core::clone::CloneToUninit") for T

where T: [Clone](https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html "trait core::clone::Clone"),

[Source](https://doc.rust-lang.org/nightly/src/core/clone.rs.html#485)[§](#method.clone_to_uninit)

#### unsafe fn [clone\_to\_uninit](https://doc.rust-lang.org/nightly/core/clone/trait.CloneToUninit.html#tymethod.clone_to_uninit)(&self, dest: [\*mut](https://doc.rust-lang.org/nightly/std/primitive.pointer.html) [u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html))

🔬This is a nightly-only experimental API. (`clone_to_uninit`)

Performs copy-assignment from `self` to `dest`. [Read more](https://doc.rust-lang.org/nightly/core/clone/trait.CloneToUninit.html#tymethod.clone_to_uninit)

[Source](https://docs.rs/gix-object/0.49.1/x86_64-unknown-linux-gnu/src/gix_object/traits/find.rs.html#308)[§](#impl-FindExt-for-T)

### impl<T> [FindExt](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html "trait gix::prelude::FindExt") for T

where T: [Find](https://docs.rs/gix/latest/gix/prelude/trait.Find.html "trait gix::prelude::Find") + ?[Sized](https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](https://docs.rs/gix-object/0.49.1/x86_64-unknown-linux-gnu/src/gix_object/traits/find.rs.html#229-233)[§](#method.find)

#### fn [find](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find)<'a>(&self, id: &[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid"), buffer: &'a mut [Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html)\>) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[Data](https://docs.rs/gix/latest/gix/diff/object/struct.Data.html "struct gix::diff::object::Data")<'a>, [Error](https://docs.rs/gix/latest/gix/diff/object/find/existing/enum.Error.html "enum gix::diff::object::find::existing::Error")\>

Like [`try_find(…)`](https://docs.rs/gix/latest/gix/prelude/trait.Find.html#tymethod.try_find "method gix::prelude::Find::try_find"), but flattens the `Result<Option<_>>` into a single `Result` making a non-existing object an error.

[Source](https://docs.rs/gix-object/0.49.1/x86_64-unknown-linux-gnu/src/gix_object/traits/find.rs.html#241-245)[§](#method.find_blob-1)

#### fn [find\_blob](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find_blob)<'a>( &self, id: &[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid"), buffer: &'a mut [Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html)\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[BlobRef](https://docs.rs/gix/latest/gix/diff/object/struct.BlobRef.html "struct gix::diff::object::BlobRef")<'a>, [Error](https://docs.rs/gix/latest/gix/diff/object/find/existing_object/enum.Error.html "enum gix::diff::object::find::existing_object::Error")\>

Like [`find(…)`](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find "method gix_object::traits::find::ext::FindExt::find::find"), but flattens the `Result<Option<_>>` into a single `Result` making a non-existing object an error while returning the desired object type.

[Source](https://docs.rs/gix-object/0.49.1/x86_64-unknown-linux-gnu/src/gix_object/traits/find.rs.html#272-276)[§](#method.find_tree-1)

#### fn [find\_tree](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find_tree)<'a>( &self, id: &[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid"), buffer: &'a mut [Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html)\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[TreeRef](https://docs.rs/gix/latest/gix/diff/object/struct.TreeRef.html "struct gix::diff::object::TreeRef")<'a>, [Error](https://docs.rs/gix/latest/gix/diff/object/find/existing_object/enum.Error.html "enum gix::diff::object::find::existing_object::Error")\>

Like [`find(…)`](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find "method gix_object::traits::find::ext::FindExt::find::find"), but flattens the `Result<Option<_>>` into a single `Result` making a non-existing object an error while returning the desired object type.

[Source](https://docs.rs/gix-object/0.49.1/x86_64-unknown-linux-gnu/src/gix_object/traits/find.rs.html#301)[§](#method.find_commit-1)

#### fn [find\_commit](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find_commit)<'a>( &self, id: &[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid"), buffer: &'a mut [Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html)\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[CommitRef](https://docs.rs/gix/latest/gix/diff/object/struct.CommitRef.html "struct gix::diff::object::CommitRef")<'a>, [Error](https://docs.rs/gix/latest/gix/diff/object/find/existing_object/enum.Error.html "enum gix::diff::object::find::existing_object::Error")\>

Like [`find(…)`](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find "method gix_object::traits::find::ext::FindExt::find::find"), but flattens the `Result<Option<_>>` into a single `Result` making a non-existing object an error while returning the desired object type.

[Source](https://docs.rs/gix-object/0.49.1/x86_64-unknown-linux-gnu/src/gix_object/traits/find.rs.html#302)[§](#method.find_tag-1)

#### fn [find\_tag](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find_tag)<'a>( &self, id: &[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid"), buffer: &'a mut [Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html)\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[TagRef](https://docs.rs/gix/latest/gix/diff/object/struct.TagRef.html "struct gix::diff::object::TagRef")<'a>, [Error](https://docs.rs/gix/latest/gix/diff/object/find/existing_object/enum.Error.html "enum gix::diff::object::find::existing_object::Error")\>

Like [`find(…)`](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find "method gix_object::traits::find::ext::FindExt::find::find"), but flattens the `Result<Option<_>>` into a single `Result` making a non-existing object an error while returning the desired object type.

[Source](https://docs.rs/gix-object/0.49.1/x86_64-unknown-linux-gnu/src/gix_object/traits/find.rs.html#303)[§](#method.find_commit_iter)

#### fn [find\_commit\_iter](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find_commit_iter)<'a>( &self, id: &[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid"), buffer: &'a mut [Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html)\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[CommitRefIter](https://docs.rs/gix/latest/gix/diff/object/struct.CommitRefIter.html "struct gix::diff::object::CommitRefIter")<'a>, [Error](https://docs.rs/gix/latest/gix/diff/object/find/existing_iter/enum.Error.html "enum gix::diff::object::find::existing_iter::Error")\>

Like [`find(…)`](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find "method gix_object::traits::find::ext::FindExt::find::find"), but flattens the `Result<Option<_>>` into a single `Result` making a non-existing object an error while returning the desired iterator type.

[Source](https://docs.rs/gix-object/0.49.1/x86_64-unknown-linux-gnu/src/gix_object/traits/find.rs.html#304)[§](#method.find_tree_iter)

#### fn [find\_tree\_iter](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find_tree_iter)<'a>( &self, id: &[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid"), buffer: &'a mut [Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html)\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[TreeRefIter](https://docs.rs/gix/latest/gix/diff/object/struct.TreeRefIter.html "struct gix::diff::object::TreeRefIter")<'a>, [Error](https://docs.rs/gix/latest/gix/diff/object/find/existing_iter/enum.Error.html "enum gix::diff::object::find::existing_iter::Error")\>

Like [`find(…)`](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find "method gix_object::traits::find::ext::FindExt::find::find"), but flattens the `Result<Option<_>>` into a single `Result` making a non-existing object an error while returning the desired iterator type.

[Source](https://docs.rs/gix-object/0.49.1/x86_64-unknown-linux-gnu/src/gix_object/traits/find.rs.html#305)[§](#method.find_tag_iter)

#### fn [find\_tag\_iter](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find_tag_iter)<'a>( &self, id: &[oid](https://docs.rs/gix/latest/gix/struct.oid.html "struct gix::oid"), buffer: &'a mut [Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")<[u8](https://doc.rust-lang.org/nightly/std/primitive.u8.html)\>, ) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<[TagRefIter](https://docs.rs/gix/latest/gix/diff/object/struct.TagRefIter.html "struct gix::diff::object::TagRefIter")<'a>, [Error](https://docs.rs/gix/latest/gix/diff/object/find/existing_iter/enum.Error.html "enum gix::diff::object::find::existing_iter::Error")\>

Like [`find(…)`](https://docs.rs/gix/latest/gix/prelude/trait.FindExt.html#method.find "method gix_object::traits::find::ext::FindExt::find::find"), but flattens the `Result<Option<_>>` into a single `Result` making a non-existing object an error while returning the desired iterator type.

[Source](https://doc.rust-lang.org/nightly/src/core/convert/mod.rs.html#774)[§](#impl-From%3CT%3E-for-T)

### impl<T> [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<T> for T

[Source](https://doc.rust-lang.org/nightly/src/core/convert/mod.rs.html#777)[§](#method.from-5)

#### fn [from](https://doc.rust-lang.org/nightly/core/convert/trait.From.html#tymethod.from)(t: T) -> T

Returns the argument unchanged.

[Source](https://doc.rust-lang.org/nightly/src/core/convert/mod.rs.html#757-759)[§](#impl-Into%3CU%3E-for-T)

### impl<T, U> [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<U> for T

where U: [From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<T>,

[Source](https://doc.rust-lang.org/nightly/src/core/convert/mod.rs.html#767)[§](#method.into)

#### fn [into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html#tymethod.into)(self) -> U

Calls `U::from(self)`.

That is, this conversion is whatever the implementation of `[From](https://doc.rust-lang.org/nightly/core/convert/trait.From.html "trait core::convert::From")<T> for U` chooses to do.

[Source](https://docs.rs/typenum/1.18.0/x86_64-unknown-linux-gnu/src/typenum/type_operators.rs.html#34)[§](#impl-Same-for-T)

### impl<T> [Same](https://docs.rs/typenum/1.18.0/x86_64-unknown-linux-gnu/typenum/type_operators/trait.Same.html "trait typenum::type_operators::Same") for T

[Source](https://docs.rs/typenum/1.18.0/x86_64-unknown-linux-gnu/src/typenum/type_operators.rs.html#35)[§](#associatedtype.Output)

#### type [Output](https://docs.rs/typenum/1.18.0/x86_64-unknown-linux-gnu/typenum/type_operators/trait.Same.html#associatedtype.Output) = T

Should always be `Self`

[Source](https://doc.rust-lang.org/nightly/src/alloc/borrow.rs.html#82-84)[§](#impl-ToOwned-for-T)

### impl<T> [ToOwned](https://doc.rust-lang.org/nightly/alloc/borrow/trait.ToOwned.html "trait alloc::borrow::ToOwned") for T

where T: [Clone](https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html "trait core::clone::Clone"),

[Source](https://doc.rust-lang.org/nightly/src/alloc/borrow.rs.html#86)[§](#associatedtype.Owned)

#### type [Owned](https://doc.rust-lang.org/nightly/alloc/borrow/trait.ToOwned.html#associatedtype.Owned) = T

The resulting type after obtaining ownership.

[Source](https://doc.rust-lang.org/nightly/src/alloc/borrow.rs.html#87)[§](#method.to_owned)

#### fn [to\_owned](https://doc.rust-lang.org/nightly/alloc/borrow/trait.ToOwned.html#tymethod.to_owned)(&self) -> T

Creates owned data from borrowed data, usually by cloning. [Read more](https://doc.rust-lang.org/nightly/alloc/borrow/trait.ToOwned.html#tymethod.to_owned)

[Source](https://doc.rust-lang.org/nightly/src/alloc/borrow.rs.html#91)[§](#method.clone_into)

#### fn [clone\_into](https://doc.rust-lang.org/nightly/alloc/borrow/trait.ToOwned.html#method.clone_into)(&self, target: [&mut T](https://doc.rust-lang.org/nightly/std/primitive.reference.html))

Uses borrowed data to replace owned data, usually by cloning. [Read more](https://doc.rust-lang.org/nightly/alloc/borrow/trait.ToOwned.html#method.clone_into)

[Source](https://doc.rust-lang.org/nightly/src/core/convert/mod.rs.html#813-815)[§](#impl-TryFrom%3CU%3E-for-T)

### impl<T, U> [TryFrom](https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html "trait core::convert::TryFrom")<U> for T

where U: [Into](https://doc.rust-lang.org/nightly/core/convert/trait.Into.html "trait core::convert::Into")<T>,

[Source](https://doc.rust-lang.org/nightly/src/core/convert/mod.rs.html#817)[§](#associatedtype.Error-1)

#### type [Error](https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html#associatedtype.Error) = [Infallible](https://doc.rust-lang.org/nightly/core/convert/enum.Infallible.html "enum core::convert::Infallible")

The type returned in the event of a conversion error.

[Source](https://doc.rust-lang.org/nightly/src/core/convert/mod.rs.html#820)[§](#method.try_from)

#### fn [try\_from](https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html#tymethod.try_from)(value: U) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<T, <T as [TryFrom](https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html "trait core::convert::TryFrom")<U>>::[Error](https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html#associatedtype.Error "type core::convert::TryFrom::Error")\>

Performs the conversion.

[Source](https://doc.rust-lang.org/nightly/src/core/convert/mod.rs.html#798-800)[§](#impl-TryInto%3CU%3E-for-T)

### impl<T, U> [TryInto](https://doc.rust-lang.org/nightly/core/convert/trait.TryInto.html "trait core::convert::TryInto")<U> for T

where U: [TryFrom](https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html "trait core::convert::TryFrom")<T>,

[Source](https://doc.rust-lang.org/nightly/src/core/convert/mod.rs.html#802)[§](#associatedtype.Error)

#### type [Error](https://doc.rust-lang.org/nightly/core/convert/trait.TryInto.html#associatedtype.Error) = <U as [TryFrom](https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html "trait core::convert::TryFrom")<T>>::[Error](https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html#associatedtype.Error "type core::convert::TryFrom::Error")

The type returned in the event of a conversion error.

[Source](https://doc.rust-lang.org/nightly/src/core/convert/mod.rs.html#805)[§](#method.try_into)

#### fn [try\_into](https://doc.rust-lang.org/nightly/core/convert/trait.TryInto.html#tymethod.try_into)(self) -> [Result](https://doc.rust-lang.org/nightly/core/result/enum.Result.html "enum core::result::Result")<U, <U as [TryFrom](https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html "trait core::convert::TryFrom")<T>>::[Error](https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html#associatedtype.Error "type core::convert::TryFrom::Error")\>

Performs the conversion.

[Source](https://docs.rs/yoke/0.7.5/x86_64-unknown-linux-gnu/src/yoke/erased.rs.html#22)[§](#impl-ErasedDestructor-for-T)

### impl<T> [ErasedDestructor](https://docs.rs/yoke/0.7.5/x86_64-unknown-linux-gnu/yoke/erased/trait.ErasedDestructor.html "trait yoke::erased::ErasedDestructor") for T

where T: 'static,

[Source](https://docs.rs/gix-object/0.49.1/x86_64-unknown-linux-gnu/src/gix_object/traits/find.rs.html#53)[§](#impl-FindObjectOrHeader-for-T)

### impl<T> [FindObjectOrHeader](https://docs.rs/gix/latest/gix/diff/object/trait.FindObjectOrHeader.html "trait gix::diff::object::FindObjectOrHeader") for T

where T: [Find](https://docs.rs/gix/latest/gix/prelude/trait.Find.html "trait gix::prelude::Find") + [Header](https://docs.rs/gix/latest/gix/diff/object/trait.FindHeader.html "trait gix::diff::object::FindHeader"),

[Source](https://docs.rs/icu_provider/1.5.0/x86_64-unknown-linux-gnu/src/icu_provider/any.rs.html#32)[§](#impl-MaybeSendSync-for-T)

### impl<T> [MaybeSendSync](https://docs.rs/icu_provider/1.5.0/x86_64-unknown-linux-gnu/icu_provider/any/trait.MaybeSendSync.html "trait icu_provider::any::MaybeSendSync") for T