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
    openStore @5 (storeName : Text) -> (store: SecretsStore);
}

struct Identity {
    id @0 : Text;
    name @1 : Text;
    email @2: Text;
}

struct Status {
    locked @0 : Bool;
    unlockedBy @1: Option(Identity);
    autolockAt @2 : Int64;
    version @3 : Text;
}

interface SecretsStore {
    status @0 () -> (status: Status);
    lock @1 ();
    unlock @2 (passphrase: Data);
    identities @3 () -> (identities: List(Identity));
    addIdentity @4 (identity: Identity, passphrase: Data);
    changePassphrase @5 (passphrase: Data);
}