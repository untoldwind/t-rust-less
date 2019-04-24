@0x981c355b6da046c4; 

interface Service {
    listStores @0 () -> (storeNames : List(Text));
}