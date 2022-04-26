const { ApiPromise, WsProvider } = require('@polkadot/api');

const {Text} = require('@polkadot/types')

const connectToLocalNode = async () => {
    const wsProvider = new WsProvider('ws://127.0.0.1:9944');

    api = await ApiPromise.create({
        provider: wsProvider,
        rpc: {
            protos: {
                getProtos: {description: "this is the description", type: "String",
                    params: [
                        {name: 'params', type: 'GetProtosParams'},

                        {name: 'at', type: 'BlockHash', isOptional: true}
                    ]},
            }
        },

        types: {
            Tags: {
                _enum: ['Code', 'Audio', 'Image']
            },

            BlockHash: 'Hash',

            GetProtosParams: {
                tags: 'Option<Vec<Tags>>',
                owner: 'Option<AccountId>',
                limit: 'u32',
                from: 'u32',
                desc: 'bool',
                metadata_keys: 'Option<Vec<String>>',
                return_owners: 'bool'
            }


        }
    });

    return api


};



// (async () => {
//     const api = await connectToLocalNode();   

//     const params = api.createType("GetProtosParams", {tags: ["Code"], owner: "5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy", limit: 10, from: 0, desc: true,
//         metadata_keys: ['A', 'A'], return_owners: true});


//     let string_json = await api.rpc.protos.getProtos(params)

//     console.log('string_json is', string_json)

// })()



module.exports.connectToLocalNode = connectToLocalNode;



