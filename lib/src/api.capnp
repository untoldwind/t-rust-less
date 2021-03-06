@0x981c355b6da046c4; 

struct Option(T) {
    union {
        some @0 : T;
        none @1 : Void;
    }
}

struct StoreConfig {
    name @0 : Text;
    storeUrl @1 : Text;
    clientId @2 : Text;
    autolockTimeoutSecs @3 : UInt64;
    defaultIdentityId @4 : Option(Text) = (none = void);
}

interface ClipboardControl {
    isDone @0 () -> (isDone: Bool);
    currentlyProviding @1 () -> (providing: Option(Text));
    provideNext @2 ();
    destroy @3 ();
}

enum EventType {
    storeUnlocked @0;
    storeLocked @1;
    secretOpened @2;
    secretVersionAdded @3;
    identityAdded @4;
    clipboardProviding @5;
    clipboardDone @6;
}

struct Event {
    type @0: EventType;
    storeName @1: Text;
    identity @2: Identity;
    secretId @3: Text;
    property @4: Text;
    blockId @5: Text;
}

interface EventSubscription {
}

interface EventHandler {
    handle @0 (event: Event);
}

struct PasswordGeneratorParam {
    union {
        chars @0: PasswordGeneratorCharsParam;
        words @1: PasswordGeneratorWordsParam;
    }

    struct PasswordGeneratorCharsParam {
        numChars @0: UInt8;
        includeUppers @1: Bool;
        includeNumbers @2: Bool;
        includeSymbols @3: Bool;
        requireUpper @4: Bool;
        requireNumber @5: Bool;
        requireSymbol @6: Bool;
        exlcudeSimilar @7: Bool;
        excludeAmbiguous @8: Bool;
   }

    struct PasswordGeneratorWordsParam {
        numWords @0: UInt8;
        delim @1: UInt32;
    }
}

interface Service {
    listStores @0 () -> (storeConfigs : List(StoreConfig));
    upsertStoreConfig @1 (storeConfig : StoreConfig);
    deleteStoreConfig @2 (storeName : Text);
    getDefaultStore @3 () -> (storeName : Option(Text));
    setDefaultStore @4 (storeName : Text);
    openStore @5 (storeName : Text) -> (store: SecretsStore);
    secretToClipboard @6 (storeName : Text, blockId : Text, properties : List(Text), displayName: Text) -> (clipboardControl: ClipboardControl);
    addEventHandler @7 (handler: EventHandler) -> (subscription: EventSubscription);
    generateId @8 () -> (id: Text);
    generatePassword @9 (param: PasswordGeneratorParam) -> (password: Text);
}

struct Identity {
    id @0 : Text;
    name @1 : Text;
    email @2: Text;
    hidden @3: Bool = false;
}

struct Status {
    locked @0 : Bool;
    unlockedBy @1: Option(Identity);
    autolockAt @2 : Int64;
    version @3 : Text;
    autolockTimeout @4: UInt64;
}

enum SecretType {
    login @0;
    note @1;
    licence @2;
    wlan @3;
    password @4;
    other @5;
}

struct SecretEntry {
    id @0 : Text;
    timestamp @1 : Int64;
    name @2 : Text;
    type @3 : SecretType;
    tags @4 : List(Text);
    urls @5 : List(Text);
    deleted @6 : Bool;
}

struct SecretListFilter {
    url @0 : Option(Text);
    tag @1 : Option(Text);
    type @2 : OptionType;
    name @3 : Option(Text);
    deleted @4 : Bool;

    # Workaround since enum can not be used as generic parameters
    struct OptionType {
        union {
            some @0 : SecretType;
            none @1 : Void;
        }
    }
}

struct SecretEntryMatch {
    entry @0 : SecretEntry;
    nameScore @1 : Int64;
    nameHighlights @2 : List(UInt64);
    urlHighlights @3 : List(UInt64);
    tagsHighlights @4 : List(UInt64);
}

struct SecretList {
    allTags @0 : List(Text);
    entries @1 : List(SecretEntryMatch);
}

struct SecretVersion {
    secretId @0 : Text;
    type @1 : SecretType;
    timestamp @2 : Int64;
    name @3 : Text;
    tags @4 : List(Text);
    urls @5 : List(Text);
    properties @6 : List(Property);
    attachments @7 : List(Attachment);
    deleted @8 : Bool;
    recipients @9 : List(Text);

    struct Property {
        key @0 : Text;
        value @1 : Text;
    }

    struct Attachment {
        name @0 : Text;
        mimeType @1 : Text;
        content @2 : Data;
    }
}

struct PasswordStrength {
    entropy @0 : Float64;
    crackTime @1 : Float64;
    crackTimeDisplay @2 : Text;
    score @3 : UInt8;
}


struct Secret {
    id @0 : Text;
    type @1 : SecretType;
    current @2 : SecretVersion;
    currentBlockId @3 : Text;
    versions @4 : List(VersionRef);
    passwordStrengths @5 : List(Estimate);

    struct Estimate {
        key @0 : Text;
        strength @1 : PasswordStrength;
    }

    struct VersionRef {
        blockId @0 : Text;
        timestamp @1 : Int64;
    }
}

interface SecretsStore {
    status @0 () -> (status: Status);
    lock @1 ();
    unlock @2 (identityId: Text, passphrase: Data);
    identities @3 () -> (identities: List(Identity));
    addIdentity @4 (identity: Identity, passphrase: Data);
    changePassphrase @5 (passphrase: Data);
    list @6 (filter: SecretListFilter) -> (list: SecretList);
    updateIndex @7 ();
    add @8 (version: SecretVersion) -> (blockId: Text);
    get @9 (id: Text) -> (secret: Secret);
    getVersion @10 (blockId: Text) -> (version: SecretVersion);
}