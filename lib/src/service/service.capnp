@0x981c355b6da046c4; 

struct StoreConfig {
    name @0 : Text;
    storeUrl @1 : Text;
    clientId @2 : Text;
    autolockTimeoutSecs @3 : UInt64;
}

struct Option(T) {
    union {
        some @0 : T;
        none @1 : Void;
    }
}

interface Service {
    listStores @0 () -> (storeNames : List(Text));
    setStoreConfig @1 (storeConfig : StoreConfig);
    getStoreConfig @2 () -> (storeConfig : StoreConfig);
    getDefaultStore @3 () -> (defaultStore : Option(Text));
    setDefaultStore @4 (defaultStore : Text);
}